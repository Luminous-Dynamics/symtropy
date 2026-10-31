//! Exact mutual-credit journaling and deterministic pooled demurrage.
//!
//! Balances are signed integer TEND units. Every committed change is a
//! double-entry transfer between named accounts. Issuance is an explicit debit
//! to an issuance account; redemption is an explicit return to it.
//!
//! Demurrage is modelled as a pooled levy on aggregate positive member
//! balances. A single fractional carry makes the total levy independent of
//! how a positive balance is fragmented across member accounts. Whole-unit
//! charges are allocated by deterministic largest remainder. Debt forgiveness
//! is not hidden in demurrage: it is a separate transfer from a funded reserve.

use std::collections::BTreeMap;

/// Denominator for rates expressed in parts per million.
pub const RATE_DENOMINATOR_PPM: u64 = 1_000_000;

/// Stable account identifier.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct AccountId(String);

impl AccountId {
    /// Construct a validated account identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, LedgerError> {
        let value = value.into();
        let normalized = value.trim();
        if normalized.is_empty() || normalized.len() > 128 {
            return Err(LedgerError::InvalidAccountId);
        }
        Ok(Self(normalized.to_owned()))
    }

    /// Return the canonical account identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Account class controls which ledger operations may affect an account.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountKind {
    /// A member's mutual-credit account.
    Member,
    /// Commons treasury; receives pooled demurrage.
    Treasury,
    /// Explicit reserve that can fund debt-relief transfers.
    DebtReliefReserve,
    /// System issuance account; a negative balance records outstanding issued units.
    Issuance,
}

/// Type of committed double-entry journal event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalKind {
    /// Ordinary member-to-member transfer.
    Transfer,
    /// Explicit issuance from the issuance account to a member.
    Issuance,
    /// Explicit redemption from a member to the issuance account.
    Redemption,
    /// Pooled demurrage transfer from member accounts to the commons treasury.
    Demurrage,
    /// Explicit debt relief funded by a debt-relief reserve.
    DebtRelief,
    /// Member or treasury transfer into a debt-relief reserve.
    ReserveFunding,
}

/// One balanced transfer in the append-only in-memory journal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalEntry {
    /// Monotonic event number within this ledger instance.
    pub sequence: u64,
    /// Account debited.
    pub from: AccountId,
    /// Account credited.
    pub to: AccountId,
    /// Positive integer amount transferred.
    pub amount: i128,
    /// Explicit economic meaning of this transfer.
    pub kind: JournalKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Account {
    kind: AccountKind,
    balance: i128,
    credit_limit: i128,
}

/// Errors returned by the accounting kernel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LedgerError {
    /// Account identifier is empty or too long.
    InvalidAccountId,
    /// Account identifier already exists.
    DuplicateAccount,
    /// A referenced account does not exist.
    UnknownAccount,
    /// A required account role does not match the operation.
    WrongAccountKind,
    /// Amount must be greater than zero.
    InvalidAmount,
    /// Rate must be strictly less than 100%.
    InvalidRate,
    /// A balance or aggregate calculation overflowed.
    Overflow,
    /// Transfer would exceed the source account's credit limit.
    CreditLimitExceeded,
    /// Redemption exceeds the member's positive balance.
    InsufficientPositiveBalance,
    /// Transfer would attempt to forgive more debt than the member owes.
    ReliefExceedsDebt,
    /// A journal sequence number is exhausted.
    SequenceExhausted,
    /// Pooled levy allocation violated its arithmetic invariant.
    InvalidLevyPlan,
    /// The ledger's aggregate balance is not conserved.
    ConservationViolation,
}

/// Double-entry ledger with explicit system accounts and exact integer balances.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ledger {
    accounts: BTreeMap<AccountId, Account>,
    entries: Vec<JournalEntry>,
    next_sequence: u64,
    demurrage_carry_ppm: u64,
    treasury: AccountId,
    issuance: AccountId,
}

impl Ledger {
    /// Create an empty ledger with a treasury and an explicit issuance account.
    pub fn new(treasury: AccountId, issuance: AccountId) -> Result<Self, LedgerError> {
        if treasury == issuance {
            return Err(LedgerError::DuplicateAccount);
        }
        let mut accounts = BTreeMap::new();
        accounts.insert(
            treasury.clone(),
            Account { kind: AccountKind::Treasury, balance: 0, credit_limit: 0 },
        );
        accounts.insert(
            issuance.clone(),
            Account { kind: AccountKind::Issuance, balance: 0, credit_limit: i128::MAX },
        );
        Ok(Self {
            accounts,
            entries: Vec::new(),
            next_sequence: 1,
            demurrage_carry_ppm: 0,
            treasury,
            issuance,
        })
    }

    /// Open a member account with a maximum permitted negative balance.
    pub fn open_member_account(
        &mut self,
        id: AccountId,
        credit_limit: i128,
    ) -> Result<(), LedgerError> {
        if credit_limit < 0 {
            return Err(LedgerError::InvalidAmount);
        }
        self.insert_account(id, Account {
            kind: AccountKind::Member,
            balance: 0,
            credit_limit,
        })
    }

    /// Open an initially empty debt-relief reserve. It cannot spend more than it holds.
    pub fn open_debt_relief_reserve(&mut self, id: AccountId) -> Result<(), LedgerError> {
        self.insert_account(id, Account {
            kind: AccountKind::DebtReliefReserve,
            balance: 0,
            credit_limit: 0,
        })
    }

    fn insert_account(&mut self, id: AccountId, account: Account) -> Result<(), LedgerError> {
        if self.accounts.contains_key(&id) {
            return Err(LedgerError::DuplicateAccount);
        }
        self.accounts.insert(id, account);
        Ok(())
    }

    /// Return the current balance of an account.
    pub fn balance(&self, id: &AccountId) -> Result<i128, LedgerError> {
        self.accounts.get(id).map(|a| a.balance).ok_or(LedgerError::UnknownAccount)
    }

    /// Return the account's configured role.
    pub fn account_kind(&self, id: &AccountId) -> Result<AccountKind, LedgerError> {
        self.accounts.get(id).map(|a| a.kind).ok_or(LedgerError::UnknownAccount)
    }

    /// Return the account identifiers in canonical sorted order.
    pub fn account_ids(&self) -> impl Iterator<Item = &AccountId> {
        self.accounts.keys()
    }

    /// Return all committed journal entries in sequence order.
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// Return the configured commons treasury account.
    pub fn treasury_id(&self) -> &AccountId {
        &self.treasury
    }

    /// Return the configured issuance account.
    pub fn issuance_id(&self) -> &AccountId {
        &self.issuance
    }

    /// Return the current pooled fractional demurrage carry (parts per million).
    pub const fn demurrage_carry_ppm(&self) -> u64 {
        self.demurrage_carry_ppm
    }

    /// Sum every account balance with checked arithmetic.
    pub fn total_balance(&self) -> Result<i128, LedgerError> {
        self.accounts.values().try_fold(0_i128, |sum, account| {
            sum.checked_add(account.balance).ok_or(LedgerError::Overflow)
        })
    }

    /// Verify the zero-sum conservation invariant across members and system accounts.
    pub fn verify_conservation(&self) -> Result<(), LedgerError> {
        if self.total_balance()? == 0 {
            Ok(())
        } else {
            Err(LedgerError::ConservationViolation)
        }
    }

    /// Explicitly issue units to a member; the issuance account records the offsetting debit.
    pub fn issue_to(&mut self, member: &AccountId, amount: i128) -> Result<(), LedgerError> {
        self.require_kind(member, AccountKind::Member)?;
        let issuance = self.issuance.clone();
        self.post_transfer(&issuance, member, amount, JournalKind::Issuance)
    }

    /// Explicitly redeem units from a member back to the issuance account.
    pub fn redeem_from(&mut self, member: &AccountId, amount: i128) -> Result<(), LedgerError> {
        self.require_kind(member, AccountKind::Member)?;
        let issuance = self.issuance.clone();
        self.post_transfer(member, &issuance, amount, JournalKind::Redemption)
    }

    /// Transfer units between member accounts.
    pub fn transfer(
        &mut self,
        from: &AccountId,
        to: &AccountId,
        amount: i128,
    ) -> Result<(), LedgerError> {
        self.require_kind(from, AccountKind::Member)?;
        self.require_kind(to, AccountKind::Member)?;
        self.post_transfer(from, to, amount, JournalKind::Transfer)
    }

    /// Fund a debt-relief reserve from a member or the treasury.
    pub fn fund_debt_relief_reserve(
        &mut self,
        from: &AccountId,
        reserve: &AccountId,
        amount: i128,
    ) -> Result<(), LedgerError> {
        self.require_kind(reserve, AccountKind::DebtReliefReserve)?;
        let from_kind = self.account_kind(from)?;
        if from_kind != AccountKind::Member && from_kind != AccountKind::Treasury {
            return Err(LedgerError::WrongAccountKind);
        }
        self.post_transfer(from, reserve, amount, JournalKind::ReserveFunding)
    }

    /// Forgive part of a member's negative balance using a funded reserve.
    ///
    /// The reserve is debited and the member's balance moves toward zero.
    /// No debt relief is created by simply reducing a negative balance.
    pub fn forgive_debt(
        &mut self,
        reserve: &AccountId,
        debtor: &AccountId,
        amount: i128,
    ) -> Result<(), LedgerError> {
        self.require_kind(reserve, AccountKind::DebtReliefReserve)?;
        self.require_kind(debtor, AccountKind::Member)?;
        if amount <= 0 {
            return Err(LedgerError::InvalidAmount);
        }
        let debt = self.balance(debtor)?;
        if debt >= 0 || amount > debt.checked_neg().ok_or(LedgerError::Overflow)? {
            return Err(LedgerError::ReliefExceedsDebt);
        }
        self.post_transfer(reserve, debtor, amount, JournalKind::DebtRelief)
    }

    /// Apply a pooled demurrage rate to positive member balances.
    ///
    /// `rate_ppm` is a per-interval rate in parts per million and must be less
    /// than 1,000,000. Fractional accrual is pooled across the member population,
    /// so splitting one balance across multiple member accounts does not change
    /// the total levy. Whole units are assigned by largest remainder, with
    /// account ID as the deterministic tie-breaker. Negative balances are not
    /// silently forgiven; use `forgive_debt` with a funded reserve.
    pub fn apply_demurrage(&mut self, rate_ppm: u64) -> Result<i128, LedgerError> {
        if rate_ppm >= RATE_DENOMINATOR_PPM {
            return Err(LedgerError::InvalidRate);
        }

        let mut total_positive = 0_u128;
        let mut positive_accounts: Vec<(AccountId, u128)> = Vec::new();
        for (id, account) in &self.accounts {
            if account.kind == AccountKind::Member && account.balance > 0 {
                let positive = u128::try_from(account.balance).map_err(|_| LedgerError::Overflow)?;
                total_positive = total_positive.checked_add(positive).ok_or(LedgerError::Overflow)?;
                positive_accounts.push((id.clone(), positive));
            }
        }

        let numerator = total_positive
            .checked_mul(rate_ppm as u128)
            .and_then(|n| n.checked_add(self.demurrage_carry_ppm as u128))
            .ok_or(LedgerError::Overflow)?;
        let levy_units = numerator / RATE_DENOMINATOR_PPM as u128;
        let new_carry = (numerator % RATE_DENOMINATOR_PPM as u128) as u64;
        let levy = i128::try_from(levy_units).map_err(|_| LedgerError::Overflow)?;

        if levy == 0 {
            self.demurrage_carry_ppm = new_carry;
            return Ok(0);
        }

        let mut shares: Vec<(AccountId, i128, u128)> = Vec::with_capacity(positive_accounts.len());
        let mut base_total = 0_u128;
        for (id, balance) in positive_accounts {
            let account_numerator = balance
                .checked_mul(rate_ppm as u128)
                .ok_or(LedgerError::Overflow)?;
            let base = account_numerator / RATE_DENOMINATOR_PPM as u128;
            let remainder = account_numerator % RATE_DENOMINATOR_PPM as u128;
            let base = i128::try_from(base).map_err(|_| LedgerError::Overflow)?;
            base_total = base_total.checked_add(base as u128).ok_or(LedgerError::Overflow)?;
            shares.push((id, base, remainder));
        }

        let extra = usize::try_from(
            levy_units
                .checked_sub(base_total)
                .ok_or(LedgerError::InvalidLevyPlan)?,
        )
            .map_err(|_| LedgerError::InvalidLevyPlan)?;
        shares.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        if extra > shares.len() {
            return Err(LedgerError::InvalidLevyPlan);
        }
        for share in shares.iter_mut().take(extra) {
            share.1 = share.1.checked_add(1).ok_or(LedgerError::Overflow)?;
        }

        let mut staged = self.clone();
        staged.demurrage_carry_ppm = new_carry;
        let treasury = staged.treasury.clone();
        for (id, charge, _) in shares {
            if charge > 0 {
                staged.post_transfer(&id, &treasury, charge, JournalKind::Demurrage)?;
            }
        }
        staged.verify_conservation()?;
        *self = staged;
        Ok(levy)
    }

    fn require_kind(&self, id: &AccountId, expected: AccountKind) -> Result<(), LedgerError> {
        if self.account_kind(id)? == expected {
            Ok(())
        } else {
            Err(LedgerError::WrongAccountKind)
        }
    }

    fn post_transfer(
        &mut self,
        from: &AccountId,
        to: &AccountId,
        amount: i128,
        kind: JournalKind,
    ) -> Result<(), LedgerError> {
        if amount <= 0 {
            return Err(LedgerError::InvalidAmount);
        }
        if from == to {
            return Err(LedgerError::WrongAccountKind);
        }

        let source = self.accounts.get(from).ok_or(LedgerError::UnknownAccount)?;
        let destination = self.accounts.get(to).ok_or(LedgerError::UnknownAccount)?;
        let new_source = source.balance.checked_sub(amount).ok_or(LedgerError::Overflow)?;
        let minimum = source.credit_limit.checked_neg().ok_or(LedgerError::Overflow)?;
        if new_source < minimum {
            return Err(LedgerError::CreditLimitExceeded);
        }
        let new_destination = destination.balance.checked_add(amount).ok_or(LedgerError::Overflow)?;
        let sequence = self.next_sequence;
        let next_sequence = sequence.checked_add(1).ok_or(LedgerError::SequenceExhausted)?;

        self.accounts.get_mut(from).ok_or(LedgerError::UnknownAccount)?.balance = new_source;
        self.accounts.get_mut(to).ok_or(LedgerError::UnknownAccount)?.balance = new_destination;
        self.entries.push(JournalEntry {
            sequence,
            from: from.clone(),
            to: to.clone(),
            amount,
            kind,
        });
        self.next_sequence = next_sequence;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> AccountId {
        AccountId::new(name).expect("valid account id")
    }

    fn setup() -> (Ledger, AccountId, AccountId, AccountId, AccountId) {
        let treasury = id("commons:treasury");
        let issuance = id("system:issuance");
        let mut ledger = Ledger::new(treasury.clone(), issuance.clone()).unwrap();
        let alice = id("alice");
        let bob = id("bob");
        ledger.open_member_account(alice.clone(), 1_000).unwrap();
        ledger.open_member_account(bob.clone(), 1_000).unwrap();
        (ledger, treasury, issuance, alice, bob)
    }

    #[test]
    fn member_transfer_is_double_entry_and_conserves_total() {
        let (mut ledger, _, _, alice, bob) = setup();
        ledger.transfer(&bob, &alice, 40).unwrap();
        assert_eq!(ledger.balance(&alice), Ok(40));
        assert_eq!(ledger.balance(&bob), Ok(-40));
        assert_eq!(ledger.verify_conservation(), Ok(()));
        assert_eq!(ledger.entries().len(), 1);
    }

    #[test]
    fn issuance_and_redemption_are_explicit_and_conserved() {
        let (mut ledger, _, issuance, alice, _) = setup();
        ledger.issue_to(&alice, 75).unwrap();
        assert_eq!(ledger.balance(&alice), Ok(75));
        assert_eq!(ledger.balance(&issuance), Ok(-75));
        ledger.redeem_from(&alice, 25).unwrap();
        assert_eq!(ledger.balance(&alice), Ok(50));
        assert_eq!(ledger.balance(&issuance), Ok(-50));
        assert_eq!(ledger.verify_conservation(), Ok(()));
        assert_eq!(ledger.entries()[0].kind, JournalKind::Issuance);
        assert_eq!(ledger.entries()[1].kind, JournalKind::Redemption);
    }

    #[test]
    fn redemption_cannot_turn_an_empty_account_into_mutual_credit_debt() {
        let (mut ledger, _, _, alice, _) = setup();
        let before = ledger.clone();
        assert_eq!(
            ledger.redeem_from(&alice, 1),
            Err(LedgerError::InsufficientPositiveBalance)
        );
        assert_eq!(ledger, before);
    }

    #[test]
    fn failed_transfer_is_transactional() {
        let (mut ledger, _, _, alice, bob) = setup();
        let before = ledger.clone();
        assert_eq!(ledger.transfer(&bob, &alice, 1_001), Err(LedgerError::CreditLimitExceeded));
        assert_eq!(ledger, before);
    }

    #[test]
    fn fractional_demurrage_does_not_charge_a_minimum_unit_per_tick() {
        let (mut ledger, treasury, _, alice, bob) = setup();
        ledger.transfer(&bob, &alice, 1).unwrap();
        assert_eq!(ledger.apply_demurrage(1_667), Ok(0));
        assert_eq!(ledger.balance(&alice), Ok(1));
        assert_eq!(ledger.balance(&treasury), Ok(0));
        assert_eq!(ledger.demurrage_carry_ppm(), 1_667);
        assert_eq!(ledger.verify_conservation(), Ok(()));
    }

    fn make_fragmentation_ledger_with(parts: &[i128]) -> (Ledger, AccountId) {
        let treasury = id("commons:treasury");
        let issuance = id("system:issuance");
        let mut ledger = Ledger::new(treasury.clone(), issuance).unwrap();
        let debtor = id("debtor");
        ledger.open_member_account(debtor.clone(), 1_000).unwrap();
        for (index, amount) in parts.iter().copied().enumerate() {
            let account = id(&format!("positive:{index:03}"));
            ledger.open_member_account(account.clone(), 1_000).unwrap();
            ledger.transfer(&debtor, &account, amount).unwrap();
        }
        (ledger, treasury)
    }

    fn make_fragmentation_ledger(split: bool) -> (Ledger, AccountId) {
        if split {
            make_fragmentation_ledger_with(&[50, 50])
        } else {
            make_fragmentation_ledger_with(&[100])
        }
    }

    #[test]
    fn pooled_demurrage_total_is_invariant_to_account_fragmentation() {
        let (mut single, single_treasury) = make_fragmentation_ledger(false);
        let (mut split, split_treasury) = make_fragmentation_ledger(true);
        for _ in 0..6 {
            single.apply_demurrage(1_667).unwrap();
            split.apply_demurrage(1_667).unwrap();
        }
        assert_eq!(single.balance(&single_treasury), Ok(1));
        assert_eq!(split.balance(&split_treasury), Ok(1));
        assert_eq!(single.verify_conservation(), Ok(()));
        assert_eq!(split.verify_conservation(), Ok(()));
    }

    #[test]
    fn pooled_levy_is_account_fragmentation_invariant_across_rates() {
        for rate in (1..=128).map(|n| n * 7_777) {
            let (mut single, single_treasury) = make_fragmentation_ledger(false);
            let (mut split, split_treasury) = make_fragmentation_ledger(true);
            for _ in 0..12 {
                single.apply_demurrage(rate).unwrap();
                split.apply_demurrage(rate).unwrap();
                assert_eq!(single.balance(&single_treasury), split.balance(&split_treasury));
                assert_eq!(single.total_balance(), split.total_balance());
                single.verify_conservation().unwrap();
                split.verify_conservation().unwrap();
            }
        }
    }

    #[test]
    fn pooled_demurrage_is_invariant_over_many_account_partitions_and_rates() {
        for seed_start in 1_u64..=64 {
            let mut seed = seed_start;
            let mut left = 100_i128;
            let mut parts = Vec::new();
            while left > 0 {
                // Deterministic bounded pseudo-random partition, no external crate.
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let amount = ((seed % 17) as i128 + 1).min(left);
                parts.push(amount);
                left -= amount;
            }

            for rate in (1..=64).map(|n| n * 13_333) {
                let (mut single, single_treasury) = make_fragmentation_ledger_with(&[100]);
                let (mut fragmented, fragmented_treasury) =
                    make_fragmentation_ledger_with(&parts);
                for _ in 0..8 {
                    let single_levy = single.apply_demurrage(rate).unwrap();
                    let fragmented_levy = fragmented.apply_demurrage(rate).unwrap();
                    assert_eq!(single_levy, fragmented_levy);
                    assert_eq!(
                        single.balance(&single_treasury),
                        fragmented.balance(&fragmented_treasury)
                    );
                    assert_eq!(single.demurrage_carry_ppm(), fragmented.demurrage_carry_ppm());
                    single.verify_conservation().unwrap();
                    fragmented.verify_conservation().unwrap();
                }
            }
        }
    }

    #[test]
    fn debt_relief_requires_and_consumes_a_funded_reserve() {
        let (mut ledger, treasury, _, alice, bob) = setup();
        ledger.transfer(&bob, &alice, 100).unwrap(); // alice +100, bob -100
        let reserve = id("care:debt-relief");
        ledger.open_debt_relief_reserve(reserve.clone()).unwrap();
        assert_eq!(
            ledger.forgive_debt(&reserve, &bob, 10),
            Err(LedgerError::CreditLimitExceeded)
        );
        ledger.fund_debt_relief_reserve(&alice, &reserve, 25).unwrap();
        ledger.forgive_debt(&reserve, &bob, 10).unwrap();
        assert_eq!(ledger.balance(&bob), Ok(-90));
        assert_eq!(ledger.balance(&reserve), Ok(15));
        assert_eq!(ledger.verify_conservation(), Ok(()));
        assert!(ledger.entries().iter().any(|entry| entry.kind == JournalKind::DebtRelief));
        assert_eq!(ledger.balance(&treasury), Ok(0));
    }

    #[test]
    fn zero_or_invalid_rates_never_mutate_ledger() {
        let (mut ledger, _, _, _, _) = setup();
        let before = ledger.clone();
        assert_eq!(ledger.apply_demurrage(RATE_DENOMINATOR_PPM), Err(LedgerError::InvalidRate));
        assert_eq!(ledger, before);
        assert_eq!(ledger.apply_demurrage(0), Ok(0));
        assert_eq!(ledger.verify_conservation(), Ok(()));
    }
}
