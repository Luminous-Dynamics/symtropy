// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Deterministic double-entry financial accounting and explicit monetary-supply authority.
//!
//! This module deliberately separates two kinds of truth:
//!
//! 1. balanced financial journal entries describing claims between accounts; and
//! 2. monetary-supply events describing explicit currency issuance/retirement.
//!
//! A balanced journal entry is therefore **not** authority to create or destroy
//! currency. Conversely, a monetary-supply event does not silently rewrite any
//! financial account balance. A later settlement/reconciliation bridge may bind the
//! two, but v0.1 keeps the authorities separate so accounting balance cannot be
//! mistaken for monetary issuance authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::economic::{ActorId, CausalId};

const MAX_ID_LEN: usize = 256;
const MAX_CURRENCIES: usize = 1024;
const MAX_ACCOUNTS: usize = 65_536;
const MAX_JOURNAL_ENTRIES: usize = 262_144;
const MAX_MONETARY_EVENTS: usize = 262_144;
const MAX_POSTINGS_PER_TRANSACTION: usize = 256;
const MAX_MINOR_UNIT_EXPONENT: u8 = 18;

macro_rules! id_type {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, FinancialError> {
                let value = value.into();
                validate_id($kind, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(CurrencyId, "currency");
id_type!(FinancialAccountId, "financial-account");
id_type!(JournalTransactionId, "journal-transaction");
id_type!(MonetaryAuthorityId, "monetary-authority");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyDefinition {
    pub currency_id: CurrencyId,
    pub monetary_authority_id: MonetaryAuthorityId,
    pub monetary_authority_actor_id: ActorId,
    /// Number of decimal places represented by one integer minor unit.
    ///
    /// Example: 2 means integer `123` represents `1.23` display units. The
    /// financial kernel performs no floating-point conversion internally.
    pub minor_unit_exponent: u8,
}

impl CurrencyDefinition {
    fn validate(&self) -> Result<(), FinancialError> {
        if self.minor_unit_exponent > MAX_MINOR_UNIT_EXPONENT {
            return Err(FinancialError::MinorUnitExponentTooLarge {
                currency_id: self.currency_id.clone(),
                max: MAX_MINOR_UNIT_EXPONENT,
                actual: self.minor_unit_exponent,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinancialAccountClass {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinancialAccount {
    pub account_id: FinancialAccountId,
    pub owner_id: ActorId,
    pub currency_id: CurrencyId,
    pub class: FinancialAccountClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PostingSide {
    Debit,
    Credit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    pub account_id: FinancialAccountId,
    pub side: PostingSide,
    /// Integer minor units of the transaction currency.
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalTransaction {
    pub transaction_id: JournalTransactionId,
    pub cause_id: CausalId,
    pub currency_id: CurrencyId,
    pub postings: Vec<Posting>,
}

impl JournalTransaction {
    pub fn new(
        transaction_id: JournalTransactionId,
        cause_id: CausalId,
        currency_id: CurrencyId,
        postings: Vec<Posting>,
    ) -> Result<Self, FinancialError> {
        let transaction = Self {
            transaction_id,
            cause_id,
            currency_id,
            postings,
        };
        transaction.validate_structure()?;
        Ok(transaction)
    }

    fn validate_structure(&self) -> Result<(), FinancialError> {
        if self.postings.len() < 2 {
            return Err(FinancialError::TooFewPostings {
                transaction_id: self.transaction_id.clone(),
            });
        }
        if self.postings.len() > MAX_POSTINGS_PER_TRANSACTION {
            return Err(FinancialError::TooManyPostings {
                transaction_id: self.transaction_id.clone(),
                max: MAX_POSTINGS_PER_TRANSACTION,
            });
        }

        let mut seen_accounts = BTreeSet::new();
        let mut debit_total = 0_u128;
        let mut credit_total = 0_u128;
        let mut previous: Option<&FinancialAccountId> = None;

        for posting in &self.postings {
            if posting.amount == 0 {
                return Err(FinancialError::ZeroPostingAmount {
                    transaction_id: self.transaction_id.clone(),
                    account_id: posting.account_id.clone(),
                });
            }
            if !seen_accounts.insert(posting.account_id.clone()) {
                return Err(FinancialError::DuplicatePostingAccount {
                    transaction_id: self.transaction_id.clone(),
                    account_id: posting.account_id.clone(),
                });
            }
            if let Some(previous_id) = previous {
                if previous_id >= &posting.account_id {
                    return Err(FinancialError::NonCanonicalPostingOrder {
                        transaction_id: self.transaction_id.clone(),
                    });
                }
            }
            previous = Some(&posting.account_id);

            match posting.side {
                PostingSide::Debit => {
                    debit_total = debit_total
                        .checked_add(u128::from(posting.amount))
                        .ok_or(FinancialError::ArithmeticOverflow)?;
                }
                PostingSide::Credit => {
                    credit_total = credit_total
                        .checked_add(u128::from(posting.amount))
                        .ok_or(FinancialError::ArithmeticOverflow)?;
                }
            }
        }

        if debit_total != credit_total {
            return Err(FinancialError::UnbalancedTransaction {
                transaction_id: self.transaction_id.clone(),
                debit_total,
                credit_total,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccountTotals {
    pub debits: u128,
    pub credits: u128,
}

impl AccountTotals {
    pub fn net(&self) -> AccountNetBalance {
        if self.debits > self.credits {
            AccountNetBalance {
                side: Some(PostingSide::Debit),
                amount: self.debits - self.credits,
            }
        } else if self.credits > self.debits {
            AccountNetBalance {
                side: Some(PostingSide::Credit),
                amount: self.credits - self.debits,
            }
        } else {
            AccountNetBalance {
                side: None,
                amount: 0,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountNetBalance {
    pub side: Option<PostingSide>,
    pub amount: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalLedgerEntry {
    pub sequence: u64,
    pub transaction: JournalTransaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonetarySupplyEvent {
    Issue {
        currency_id: CurrencyId,
        authority_id: MonetaryAuthorityId,
        amount: u64,
        beneficiary_actor_id: ActorId,
        cause_id: CausalId,
    },
    Retire {
        currency_id: CurrencyId,
        authority_id: MonetaryAuthorityId,
        amount: u64,
        source_actor_id: ActorId,
        cause_id: CausalId,
    },
}

impl MonetarySupplyEvent {
    fn currency_id(&self) -> &CurrencyId {
        match self {
            Self::Issue { currency_id, .. } | Self::Retire { currency_id, .. } => currency_id,
        }
    }

    fn authority_id(&self) -> &MonetaryAuthorityId {
        match self {
            Self::Issue { authority_id, .. } | Self::Retire { authority_id, .. } => authority_id,
        }
    }

    fn amount(&self) -> u64 {
        match self {
            Self::Issue { amount, .. } | Self::Retire { amount, .. } => *amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonetaryLedgerEntry {
    pub sequence: u64,
    pub event: MonetarySupplyEvent,
}

/// Combined financial representation with two independent append-only histories.
///
/// `journal_entries` establish double-entry accounting truth. `monetary_entries`
/// establish aggregate declared currency supply. Neither history mutates the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinancialBook {
    currencies: BTreeMap<CurrencyId, CurrencyDefinition>,
    accounts: BTreeMap<FinancialAccountId, FinancialAccount>,
    balances: BTreeMap<FinancialAccountId, AccountTotals>,
    journal_entries: Vec<JournalLedgerEntry>,
    monetary_supply: BTreeMap<CurrencyId, u64>,
    monetary_entries: Vec<MonetaryLedgerEntry>,
}

impl FinancialBook {
    pub fn new(
        currencies: impl IntoIterator<Item = CurrencyDefinition>,
        accounts: impl IntoIterator<Item = FinancialAccount>,
    ) -> Result<Self, FinancialError> {
        let mut currency_map = BTreeMap::new();
        for currency in currencies {
            currency.validate()?;
            let id = currency.currency_id.clone();
            if currency_map.insert(id.clone(), currency).is_some() {
                return Err(FinancialError::DuplicateCurrency { currency_id: id });
            }
        }
        if currency_map.is_empty() {
            return Err(FinancialError::NoCurrencies);
        }
        if currency_map.len() > MAX_CURRENCIES {
            return Err(FinancialError::ModelTooLarge);
        }

        let mut account_map = BTreeMap::new();
        for account in accounts {
            if !currency_map.contains_key(&account.currency_id) {
                return Err(FinancialError::UnknownCurrency {
                    currency_id: account.currency_id.clone(),
                });
            }
            let id = account.account_id.clone();
            if account_map.insert(id.clone(), account).is_some() {
                return Err(FinancialError::DuplicateAccount { account_id: id });
            }
        }
        if account_map.is_empty() {
            return Err(FinancialError::NoAccounts);
        }
        if account_map.len() > MAX_ACCOUNTS {
            return Err(FinancialError::ModelTooLarge);
        }

        let balances = account_map
            .keys()
            .cloned()
            .map(|id| (id, AccountTotals::default()))
            .collect();
        let monetary_supply = currency_map
            .keys()
            .cloned()
            .map(|id| (id, 0_u64))
            .collect();

        Ok(Self {
            currencies: currency_map,
            accounts: account_map,
            balances,
            journal_entries: Vec::new(),
            monetary_supply,
            monetary_entries: Vec::new(),
        })
    }

    pub fn from_history(
        currencies: impl IntoIterator<Item = CurrencyDefinition>,
        accounts: impl IntoIterator<Item = FinancialAccount>,
        journal_entries: Vec<JournalLedgerEntry>,
        monetary_entries: Vec<MonetaryLedgerEntry>,
    ) -> Result<Self, FinancialError> {
        let mut book = Self::new(currencies, accounts)?;
        if journal_entries.len() > MAX_JOURNAL_ENTRIES
            || monetary_entries.len() > MAX_MONETARY_EVENTS
        {
            return Err(FinancialError::ModelTooLarge);
        }
        book.balances = replay_journal(&book.currencies, &book.accounts, &journal_entries)?;
        book.monetary_supply = replay_monetary(&book.currencies, &monetary_entries)?;
        book.journal_entries = journal_entries;
        book.monetary_entries = monetary_entries;
        Ok(book)
    }

    pub fn validate(&self) -> Result<(), FinancialError> {
        if self.currencies.is_empty() {
            return Err(FinancialError::NoCurrencies);
        }
        if self.accounts.is_empty() {
            return Err(FinancialError::NoAccounts);
        }
        if self.currencies.len() > MAX_CURRENCIES
            || self.accounts.len() > MAX_ACCOUNTS
            || self.journal_entries.len() > MAX_JOURNAL_ENTRIES
            || self.monetary_entries.len() > MAX_MONETARY_EVENTS
        {
            return Err(FinancialError::ModelTooLarge);
        }
        for currency in self.currencies.values() {
            currency.validate()?;
        }
        for account in self.accounts.values() {
            if !self.currencies.contains_key(&account.currency_id) {
                return Err(FinancialError::UnknownCurrency {
                    currency_id: account.currency_id.clone(),
                });
            }
        }
        let replayed_balances =
            replay_journal(&self.currencies, &self.accounts, &self.journal_entries)?;
        if replayed_balances != self.balances {
            return Err(FinancialError::JournalHistoryMismatch);
        }
        let replayed_supply = replay_monetary(&self.currencies, &self.monetary_entries)?;
        if replayed_supply != self.monetary_supply {
            return Err(FinancialError::MonetaryHistoryMismatch);
        }
        Ok(())
    }

    pub fn currency(&self, currency_id: &CurrencyId) -> Option<&CurrencyDefinition> {
        self.currencies.get(currency_id)
    }

    pub fn account(&self, account_id: &FinancialAccountId) -> Option<&FinancialAccount> {
        self.accounts.get(account_id)
    }

    pub fn account_totals(&self, account_id: &FinancialAccountId) -> Option<AccountTotals> {
        self.balances.get(account_id).copied()
    }

    pub fn journal_entries(&self) -> &[JournalLedgerEntry] {
        &self.journal_entries
    }

    pub fn monetary_entries(&self) -> &[MonetaryLedgerEntry] {
        &self.monetary_entries
    }

    pub fn monetary_supply(&self, currency_id: &CurrencyId) -> Option<u64> {
        self.monetary_supply.get(currency_id).copied()
    }

    /// Append one balanced journal transaction atomically.
    ///
    /// This function never changes declared monetary supply.
    pub fn post_transaction(
        &mut self,
        transaction: JournalTransaction,
    ) -> Result<(), FinancialError> {
        if self.journal_entries.len() >= MAX_JOURNAL_ENTRIES {
            return Err(FinancialError::ModelTooLarge);
        }
        validate_transaction_against_book(&self.currencies, &self.accounts, &transaction)?;
        if self
            .journal_entries
            .iter()
            .any(|entry| entry.transaction.transaction_id == transaction.transaction_id)
        {
            return Err(FinancialError::DuplicateTransaction {
                transaction_id: transaction.transaction_id.clone(),
            });
        }

        let mut candidate_balances = self.balances.clone();
        apply_transaction(&mut candidate_balances, &transaction)?;
        let sequence = next_sequence(self.journal_entries.len())?;
        let mut candidate_entries = self.journal_entries.clone();
        candidate_entries.push(JournalLedgerEntry {
            sequence,
            transaction,
        });

        let replayed = replay_journal(&self.currencies, &self.accounts, &candidate_entries)?;
        if replayed != candidate_balances {
            return Err(FinancialError::JournalHistoryMismatch);
        }

        self.balances = candidate_balances;
        self.journal_entries = candidate_entries;
        Ok(())
    }

    /// Increase aggregate declared supply only under the currency's exact registered
    /// monetary authority. No financial account is modified here.
    pub fn issue_currency(
        &mut self,
        currency_id: CurrencyId,
        authority_id: MonetaryAuthorityId,
        amount: u64,
        beneficiary_actor_id: ActorId,
        cause_id: CausalId,
    ) -> Result<(), FinancialError> {
        self.apply_monetary_event(MonetarySupplyEvent::Issue {
            currency_id,
            authority_id,
            amount,
            beneficiary_actor_id,
            cause_id,
        })
    }

    /// Decrease aggregate declared supply only under the exact registered monetary
    /// authority. No financial account is modified here.
    pub fn retire_currency(
        &mut self,
        currency_id: CurrencyId,
        authority_id: MonetaryAuthorityId,
        amount: u64,
        source_actor_id: ActorId,
        cause_id: CausalId,
    ) -> Result<(), FinancialError> {
        self.apply_monetary_event(MonetarySupplyEvent::Retire {
            currency_id,
            authority_id,
            amount,
            source_actor_id,
            cause_id,
        })
    }

    fn apply_monetary_event(&mut self, event: MonetarySupplyEvent) -> Result<(), FinancialError> {
        if self.monetary_entries.len() >= MAX_MONETARY_EVENTS {
            return Err(FinancialError::ModelTooLarge);
        }
        validate_monetary_event(&self.currencies, &event)?;

        let mut candidate_supply = self.monetary_supply.clone();
        apply_supply_event(&mut candidate_supply, &event)?;
        let sequence = next_sequence(self.monetary_entries.len())?;
        let mut candidate_entries = self.monetary_entries.clone();
        candidate_entries.push(MonetaryLedgerEntry { sequence, event });

        let replayed = replay_monetary(&self.currencies, &candidate_entries)?;
        if replayed != candidate_supply {
            return Err(FinancialError::MonetaryHistoryMismatch);
        }

        self.monetary_supply = candidate_supply;
        self.monetary_entries = candidate_entries;
        Ok(())
    }
}

fn next_sequence(len: usize) -> Result<u64, FinancialError> {
    u64::try_from(len)
        .map_err(|_| FinancialError::ArithmeticOverflow)?
        .checked_add(1)
        .ok_or(FinancialError::ArithmeticOverflow)
}

fn validate_transaction_against_book(
    currencies: &BTreeMap<CurrencyId, CurrencyDefinition>,
    accounts: &BTreeMap<FinancialAccountId, FinancialAccount>,
    transaction: &JournalTransaction,
) -> Result<(), FinancialError> {
    transaction.validate_structure()?;
    if !currencies.contains_key(&transaction.currency_id) {
        return Err(FinancialError::UnknownCurrency {
            currency_id: transaction.currency_id.clone(),
        });
    }
    for posting in &transaction.postings {
        let account = accounts
            .get(&posting.account_id)
            .ok_or_else(|| FinancialError::UnknownAccount {
                account_id: posting.account_id.clone(),
            })?;
        if account.currency_id != transaction.currency_id {
            return Err(FinancialError::AccountCurrencyMismatch {
                transaction_id: transaction.transaction_id.clone(),
                account_id: account.account_id.clone(),
                transaction_currency_id: transaction.currency_id.clone(),
                account_currency_id: account.currency_id.clone(),
            });
        }
    }
    Ok(())
}

fn apply_transaction(
    balances: &mut BTreeMap<FinancialAccountId, AccountTotals>,
    transaction: &JournalTransaction,
) -> Result<(), FinancialError> {
    for posting in &transaction.postings {
        let totals = balances
            .get_mut(&posting.account_id)
            .ok_or_else(|| FinancialError::UnknownAccount {
                account_id: posting.account_id.clone(),
            })?;
        match posting.side {
            PostingSide::Debit => {
                totals.debits = totals
                    .debits
                    .checked_add(u128::from(posting.amount))
                    .ok_or(FinancialError::ArithmeticOverflow)?;
            }
            PostingSide::Credit => {
                totals.credits = totals
                    .credits
                    .checked_add(u128::from(posting.amount))
                    .ok_or(FinancialError::ArithmeticOverflow)?;
            }
        }
    }
    Ok(())
}

fn replay_journal(
    currencies: &BTreeMap<CurrencyId, CurrencyDefinition>,
    accounts: &BTreeMap<FinancialAccountId, FinancialAccount>,
    entries: &[JournalLedgerEntry],
) -> Result<BTreeMap<FinancialAccountId, AccountTotals>, FinancialError> {
    if entries.len() > MAX_JOURNAL_ENTRIES {
        return Err(FinancialError::ModelTooLarge);
    }
    let mut balances: BTreeMap<_, _> = accounts
        .keys()
        .cloned()
        .map(|id| (id, AccountTotals::default()))
        .collect();
    let mut transaction_ids = BTreeSet::new();

    for (index, entry) in entries.iter().enumerate() {
        let expected = next_sequence(index)?;
        if entry.sequence != expected {
            return Err(FinancialError::InvalidJournalSequence {
                expected,
                actual: entry.sequence,
            });
        }
        validate_transaction_against_book(currencies, accounts, &entry.transaction)?;
        if !transaction_ids.insert(entry.transaction.transaction_id.clone()) {
            return Err(FinancialError::DuplicateTransaction {
                transaction_id: entry.transaction.transaction_id.clone(),
            });
        }
        apply_transaction(&mut balances, &entry.transaction)?;
    }
    Ok(balances)
}

fn validate_monetary_event(
    currencies: &BTreeMap<CurrencyId, CurrencyDefinition>,
    event: &MonetarySupplyEvent,
) -> Result<(), FinancialError> {
    if event.amount() == 0 {
        return Err(FinancialError::ZeroMonetaryAmount);
    }
    let currency = currencies
        .get(event.currency_id())
        .ok_or_else(|| FinancialError::UnknownCurrency {
            currency_id: event.currency_id().clone(),
        })?;
    if &currency.monetary_authority_id != event.authority_id() {
        return Err(FinancialError::WrongMonetaryAuthority {
            currency_id: currency.currency_id.clone(),
            expected: currency.monetary_authority_id.clone(),
            actual: event.authority_id().clone(),
        });
    }
    Ok(())
}

fn apply_supply_event(
    supply: &mut BTreeMap<CurrencyId, u64>,
    event: &MonetarySupplyEvent,
) -> Result<(), FinancialError> {
    let current = supply
        .get_mut(event.currency_id())
        .ok_or_else(|| FinancialError::UnknownCurrency {
            currency_id: event.currency_id().clone(),
        })?;
    match event {
        MonetarySupplyEvent::Issue { amount, .. } => {
            *current = current
                .checked_add(*amount)
                .ok_or(FinancialError::ArithmeticOverflow)?;
        }
        MonetarySupplyEvent::Retire { amount, .. } => {
            if *amount > *current {
                return Err(FinancialError::RetirementExceedsSupply {
                    currency_id: event.currency_id().clone(),
                    current_supply: *current,
                    requested: *amount,
                });
            }
            *current = current
                .checked_sub(*amount)
                .ok_or(FinancialError::ArithmeticOverflow)?;
        }
    }
    Ok(())
}

fn replay_monetary(
    currencies: &BTreeMap<CurrencyId, CurrencyDefinition>,
    entries: &[MonetaryLedgerEntry],
) -> Result<BTreeMap<CurrencyId, u64>, FinancialError> {
    if entries.len() > MAX_MONETARY_EVENTS {
        return Err(FinancialError::ModelTooLarge);
    }
    let mut supply: BTreeMap<_, _> = currencies
        .keys()
        .cloned()
        .map(|id| (id, 0_u64))
        .collect();
    for (index, entry) in entries.iter().enumerate() {
        let expected = next_sequence(index)?;
        if entry.sequence != expected {
            return Err(FinancialError::InvalidMonetarySequence {
                expected,
                actual: entry.sequence,
            });
        }
        validate_monetary_event(currencies, &entry.event)?;
        apply_supply_event(&mut supply, &entry.event)?;
    }
    Ok(supply)
}

fn validate_id(kind: &'static str, value: &str) -> Result<(), FinancialError> {
    if value.is_empty() {
        return Err(FinancialError::EmptyId { kind });
    }
    if value.len() > MAX_ID_LEN {
        return Err(FinancialError::IdTooLong {
            kind,
            max_len: MAX_ID_LEN,
        });
    }
    if value.trim() != value {
        return Err(FinancialError::IdHasSurroundingWhitespace { kind });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinancialError {
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
    NoCurrencies,
    NoAccounts,
    DuplicateCurrency {
        currency_id: CurrencyId,
    },
    UnknownCurrency {
        currency_id: CurrencyId,
    },
    DuplicateAccount {
        account_id: FinancialAccountId,
    },
    UnknownAccount {
        account_id: FinancialAccountId,
    },
    MinorUnitExponentTooLarge {
        currency_id: CurrencyId,
        max: u8,
        actual: u8,
    },
    TooFewPostings {
        transaction_id: JournalTransactionId,
    },
    TooManyPostings {
        transaction_id: JournalTransactionId,
        max: usize,
    },
    ZeroPostingAmount {
        transaction_id: JournalTransactionId,
        account_id: FinancialAccountId,
    },
    DuplicatePostingAccount {
        transaction_id: JournalTransactionId,
        account_id: FinancialAccountId,
    },
    NonCanonicalPostingOrder {
        transaction_id: JournalTransactionId,
    },
    UnbalancedTransaction {
        transaction_id: JournalTransactionId,
        debit_total: u128,
        credit_total: u128,
    },
    AccountCurrencyMismatch {
        transaction_id: JournalTransactionId,
        account_id: FinancialAccountId,
        transaction_currency_id: CurrencyId,
        account_currency_id: CurrencyId,
    },
    DuplicateTransaction {
        transaction_id: JournalTransactionId,
    },
    WrongMonetaryAuthority {
        currency_id: CurrencyId,
        expected: MonetaryAuthorityId,
        actual: MonetaryAuthorityId,
    },
    ZeroMonetaryAmount,
    RetirementExceedsSupply {
        currency_id: CurrencyId,
        current_supply: u64,
        requested: u64,
    },
    InvalidJournalSequence {
        expected: u64,
        actual: u64,
    },
    InvalidMonetarySequence {
        expected: u64,
        actual: u64,
    },
    JournalHistoryMismatch,
    MonetaryHistoryMismatch,
    ArithmeticOverflow,
    ModelTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn transaction_id(value: &str) -> JournalTransactionId {
        JournalTransactionId::new(value).unwrap()
    }

    fn usd() -> CurrencyDefinition {
        CurrencyDefinition {
            currency_id: currency("USD-test"),
            monetary_authority_id: authority("usd-authority"),
            monetary_authority_actor_id: actor("treasury"),
            minor_unit_exponent: 2,
        }
    }

    fn eur() -> CurrencyDefinition {
        CurrencyDefinition {
            currency_id: currency("EUR-test"),
            monetary_authority_id: authority("eur-authority"),
            monetary_authority_actor_id: actor("euro-authority-actor"),
            minor_unit_exponent: 2,
        }
    }

    fn account(
        id: &str,
        owner: &str,
        currency_id: CurrencyId,
        class: FinancialAccountClass,
    ) -> FinancialAccount {
        FinancialAccount {
            account_id: account_id(id),
            owner_id: actor(owner),
            currency_id,
            class,
        }
    }

    fn book() -> FinancialBook {
        FinancialBook::new(
            vec![usd(), eur()],
            vec![
                account(
                    "alice-cash",
                    "alice",
                    currency("USD-test"),
                    FinancialAccountClass::Asset,
                ),
                account(
                    "bob-cash",
                    "bob",
                    currency("USD-test"),
                    FinancialAccountClass::Asset,
                ),
                account(
                    "sales-revenue",
                    "shop",
                    currency("USD-test"),
                    FinancialAccountClass::Revenue,
                ),
                account(
                    "euro-cash",
                    "alice",
                    currency("EUR-test"),
                    FinancialAccountClass::Asset,
                ),
            ],
        )
        .unwrap()
    }

    fn transfer_transaction(id: &str, amount: u64) -> JournalTransaction {
        JournalTransaction::new(
            transaction_id(id),
            cause("sale-001"),
            currency("USD-test"),
            vec![
                Posting {
                    account_id: account_id("alice-cash"),
                    side: PostingSide::Debit,
                    amount,
                },
                Posting {
                    account_id: account_id("sales-revenue"),
                    side: PostingSide::Credit,
                    amount,
                },
            ],
        )
        .unwrap()
    }

    #[test]
    fn balanced_transaction_updates_only_financial_history() {
        let mut book = book();
        book.post_transaction(transfer_transaction("tx-001", 250))
            .unwrap();

        assert_eq!(
            book.account_totals(&account_id("alice-cash")).unwrap(),
            AccountTotals {
                debits: 250,
                credits: 0
            }
        );
        assert_eq!(
            book.account_totals(&account_id("sales-revenue")).unwrap(),
            AccountTotals {
                debits: 0,
                credits: 250
            }
        );
        assert_eq!(book.monetary_supply(&currency("USD-test")), Some(0));
        assert!(book.monetary_entries().is_empty());
    }

    #[test]
    fn unbalanced_transaction_is_rejected_before_book_mutation() {
        let error = JournalTransaction::new(
            transaction_id("tx-bad"),
            cause("bad"),
            currency("USD-test"),
            vec![
                Posting {
                    account_id: account_id("alice-cash"),
                    side: PostingSide::Debit,
                    amount: 100,
                },
                Posting {
                    account_id: account_id("sales-revenue"),
                    side: PostingSide::Credit,
                    amount: 99,
                },
            ],
        )
        .unwrap_err();
        assert!(matches!(error, FinancialError::UnbalancedTransaction { .. }));
    }

    #[test]
    fn cross_currency_posting_fails_atomically() {
        let mut book = book();
        let before = book.clone();
        let transaction = JournalTransaction::new(
            transaction_id("tx-cross"),
            cause("exchange-not-qualified"),
            currency("USD-test"),
            vec![
                Posting {
                    account_id: account_id("euro-cash"),
                    side: PostingSide::Debit,
                    amount: 100,
                },
                Posting {
                    account_id: account_id("sales-revenue"),
                    side: PostingSide::Credit,
                    amount: 100,
                },
            ],
        )
        .unwrap();
        let error = book.post_transaction(transaction).unwrap_err();
        assert!(matches!(error, FinancialError::AccountCurrencyMismatch { .. }));
        assert_eq!(book, before);
    }

    #[test]
    fn duplicate_transaction_id_fails_closed() {
        let mut book = book();
        book.post_transaction(transfer_transaction("tx-001", 250))
            .unwrap();
        let before = book.clone();
        let error = book
            .post_transaction(transfer_transaction("tx-001", 250))
            .unwrap_err();
        assert!(matches!(error, FinancialError::DuplicateTransaction { .. }));
        assert_eq!(book, before);
    }

    #[test]
    fn monetary_issue_requires_exact_authority_and_does_not_touch_accounts() {
        let mut book = book();
        let before_balances = book.balances.clone();

        let error = book
            .issue_currency(
                currency("USD-test"),
                authority("wrong-authority"),
                1_000,
                actor("alice"),
                cause("issue-001"),
            )
            .unwrap_err();
        assert!(matches!(error, FinancialError::WrongMonetaryAuthority { .. }));
        assert_eq!(book.monetary_supply(&currency("USD-test")), Some(0));

        book.issue_currency(
            currency("USD-test"),
            authority("usd-authority"),
            1_000,
            actor("alice"),
            cause("issue-002"),
        )
        .unwrap();
        assert_eq!(book.monetary_supply(&currency("USD-test")), Some(1_000));
        assert_eq!(book.balances, before_balances);
    }

    #[test]
    fn monetary_retirement_cannot_exceed_declared_supply() {
        let mut book = book();
        book.issue_currency(
            currency("USD-test"),
            authority("usd-authority"),
            1_000,
            actor("alice"),
            cause("issue-001"),
        )
        .unwrap();
        let before = book.clone();
        let error = book
            .retire_currency(
                currency("USD-test"),
                authority("usd-authority"),
                1_001,
                actor("alice"),
                cause("retire-001"),
            )
            .unwrap_err();
        assert!(matches!(error, FinancialError::RetirementExceedsSupply { .. }));
        assert_eq!(book, before);
    }

    #[test]
    fn issue_then_retire_replays_exact_supply() {
        let mut book = book();
        book.issue_currency(
            currency("USD-test"),
            authority("usd-authority"),
            1_000,
            actor("alice"),
            cause("issue-001"),
        )
        .unwrap();
        book.retire_currency(
            currency("USD-test"),
            authority("usd-authority"),
            250,
            actor("alice"),
            cause("retire-001"),
        )
        .unwrap();
        assert_eq!(book.monetary_supply(&currency("USD-test")), Some(750));
        book.validate().unwrap();
    }

    #[test]
    fn complete_history_reconstructs_identical_book() {
        let mut original = book();
        original
            .post_transaction(transfer_transaction("tx-001", 250))
            .unwrap();
        original
            .issue_currency(
                currency("USD-test"),
                authority("usd-authority"),
                1_000,
                actor("alice"),
                cause("issue-001"),
            )
            .unwrap();

        let rebuilt = FinancialBook::from_history(
            original.currencies.values().cloned().collect::<Vec<_>>(),
            original.accounts.values().cloned().collect::<Vec<_>>(),
            original.journal_entries.clone(),
            original.monetary_entries.clone(),
        )
        .unwrap();
        assert_eq!(rebuilt, original);
    }

    #[test]
    fn corrupted_materialized_balance_is_detected() {
        let mut book = book();
        book.post_transaction(transfer_transaction("tx-001", 250))
            .unwrap();
        book.balances.get_mut(&account_id("alice-cash")).unwrap().debits = 249;
        assert_eq!(book.validate(), Err(FinancialError::JournalHistoryMismatch));
    }

    #[test]
    fn corrupted_materialized_supply_is_detected() {
        let mut book = book();
        book.issue_currency(
            currency("USD-test"),
            authority("usd-authority"),
            1_000,
            actor("alice"),
            cause("issue-001"),
        )
        .unwrap();
        *book.monetary_supply.get_mut(&currency("USD-test")).unwrap() = 999;
        assert_eq!(book.validate(), Err(FinancialError::MonetaryHistoryMismatch));
    }

    #[test]
    fn account_net_balance_preserves_debit_credit_orientation() {
        let totals = AccountTotals {
            debits: 200,
            credits: 75,
        };
        assert_eq!(
            totals.net(),
            AccountNetBalance {
                side: Some(PostingSide::Debit),
                amount: 125
            }
        );
    }

    #[test]
    fn canonical_posting_order_is_required() {
        let error = JournalTransaction::new(
            transaction_id("tx-order"),
            cause("order"),
            currency("USD-test"),
            vec![
                Posting {
                    account_id: account_id("sales-revenue"),
                    side: PostingSide::Credit,
                    amount: 100,
                },
                Posting {
                    account_id: account_id("alice-cash"),
                    side: PostingSide::Debit,
                    amount: 100,
                },
            ],
        )
        .unwrap_err();
        assert!(matches!(error, FinancialError::NonCanonicalPostingOrder { .. }));
    }
}
