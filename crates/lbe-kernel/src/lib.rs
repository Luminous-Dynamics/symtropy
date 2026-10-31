//! LBE kernel: deterministic, dependency-free primitives for rights-first provisioning.
//!
//! This crate intentionally does not depend on Bevy, Holochain, Symthaea, a
//! network, a database, or an external identity service. It is a pure model
//! whose invariants can be tested independently.
//!
//! Security boundary: `AuthenticatedSubject::from_verified_adapter` must
//! only be called by an adapter after authenticating the subject. This crate
//! does not itself authenticate people, store sensitive case narratives, or
//! make real-world eligibility decisions.
//!
//! Most importantly, the need-intake API has no fields for wealth, reputation,
//! MYCEL, SAP, TEND, Phi, contribution history, or civic participation.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod ledger;
pub mod needs;

pub use ledger::{
    AccountId, AccountKind, JournalEntry, JournalKind, Ledger, LedgerError,
    RATE_DENOMINATOR_PPM,
};
pub use needs::{
    Allocation, AllocationPolicy, AuthenticatedSubject, NeedCase, NeedCategory,
    NeedError, NeedId, NeedIntake, NeedRequest, NeedSeverity,
};
