//! Bilateral consent state machine for matches and service exchanges.
//!
//! This is a pure model. The host adapter must authenticate each action author
//! and recompute the canonical proposal digest from immutable terms before
//! calling these methods. A caller-provided digest is not itself a signature.
//!
//! Consent is independent of reputation, Phi, contribution, gratitude and
//! reciprocity. Acceptance requires two distinct parties to accept the same
//! proposal. Pending consent may expire; either party may revoke before
//! settlement. A changed proposal requires a new digest and a new consent pair.

use crate::needs::AuthenticatedSubject;

/// A 32-byte digest of the canonical, immutable proposal terms.
///
/// The integration adapter is responsible for canonical serialization and
/// cryptographic hashing. The kernel only checks exact digest equality.
pub type ProposalDigest = [u8; 32];

/// State of a two-party consent process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsentStatus {
    /// Neither party has accepted the proposal yet.
    Pending,
    /// Both distinct parties accepted the same proposal digest.
    Accepted,
    /// At least one party explicitly declined before mutual acceptance.
    Declined,
    /// The pending proposal passed its deadline.
    Expired,
    /// A participant revoked consent before settlement.
    Revoked,
}

/// Errors returned by the bilateral consent state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsentError {
    /// Both parties must be distinct.
    SameParty,
    /// Expiry must be later than the proposal creation time.
    InvalidExpiry,
    /// The caller supplied a digest other than the immutable proposal digest.
    ProposalMismatch,
    /// The authenticated actor is not one of the two proposal parties.
    NotParticipant,
    /// The proposal has already been declined.
    AlreadyDeclined,
    /// The proposal has expired.
    Expired,
    /// Consent has been revoked.
    Revoked,
    /// Consent is no longer revocable because the operation has settled.
    AlreadySettled,
    /// A caller attempted to settle before both participants accepted.
    NotAccepted,
}

/// Bilateral consent bound to a fixed proposal digest and two authenticated parties.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BilateralConsent {
    digest: ProposalDigest,
    parties: [AuthenticatedSubject; 2],
    accepted: [bool; 2],
    created_at: u64,
    expires_at: u64,
    status: ConsentStatus,
    settled: bool,
}

impl BilateralConsent {
    /// Create a pending consent process.
    pub fn new(
        digest: ProposalDigest,
        first_party: AuthenticatedSubject,
        second_party: AuthenticatedSubject,
        created_at: u64,
        expires_at: u64,
    ) -> Result<Self, ConsentError> {
        if first_party == second_party {
            return Err(ConsentError::SameParty);
        }
        if expires_at <= created_at {
            return Err(ConsentError::InvalidExpiry);
        }
        Ok(Self {
            digest,
            parties: [first_party, second_party],
            accepted: [false, false],
            created_at,
            expires_at,
            status: ConsentStatus::Pending,
            settled: false,
        })
    }

    /// Return the immutable proposal digest.
    pub const fn proposal_digest(&self) -> &ProposalDigest {
        &self.digest
    }

    /// Return the creation time.
    pub const fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Return the expiry time.
    pub const fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Return current consent status.
    pub const fn status(&self) -> ConsentStatus {
        self.status
    }

    /// Return whether both parties have accepted and settlement is permitted.
    pub const fn can_settle(&self) -> bool {
        matches!(self.status, ConsentStatus::Accepted) && !self.settled
    }

    /// Return whether settlement has been recorded in this model.
    pub const fn is_settled(&self) -> bool {
        self.settled
    }

    /// Return whether the specified authenticated participant has accepted.
    pub fn has_accepted(&self, party: &AuthenticatedSubject) -> bool {
        self.party_index(party).is_some_and(|index| self.accepted[index])
    }

    /// Record acceptance by one participant over the exact immutable proposal digest.
    ///
    /// Repeated acceptance by the same participant is idempotent. A proposal
    /// becomes Accepted only after both distinct parties have accepted it.
    pub fn accept(
        &mut self,
        actor: &AuthenticatedSubject,
        digest: &ProposalDigest,
        now: u64,
    ) -> Result<ConsentStatus, ConsentError> {
        self.validate_actor_and_digest(actor, digest)?;
        match self.status {
            ConsentStatus::Accepted => return Ok(ConsentStatus::Accepted),
            ConsentStatus::Declined => return Err(ConsentError::AlreadyDeclined),
            ConsentStatus::Expired => return Err(ConsentError::Expired),
            ConsentStatus::Revoked => return Err(ConsentError::Revoked),
            ConsentStatus::Pending => {}
        }
        if now >= self.expires_at {
            self.status = ConsentStatus::Expired;
            return Err(ConsentError::Expired);
        }

        let index = self.party_index(actor).ok_or(ConsentError::NotParticipant)?;
        self.accepted[index] = true;
        if self.accepted[0] && self.accepted[1] {
            self.status = ConsentStatus::Accepted;
        }
        Ok(self.status)
    }

    /// Record an explicit decline by either party while consent is pending.
    pub fn decline(
        &mut self,
        actor: &AuthenticatedSubject,
        digest: &ProposalDigest,
        now: u64,
    ) -> Result<ConsentStatus, ConsentError> {
        self.validate_actor_and_digest(actor, digest)?;
        match self.status {
            ConsentStatus::Accepted => return Err(ConsentError::NotAccepted),
            ConsentStatus::Declined => return Ok(ConsentStatus::Declined),
            ConsentStatus::Expired => return Err(ConsentError::Expired),
            ConsentStatus::Revoked => return Err(ConsentError::Revoked),
            ConsentStatus::Pending => {}
        }
        if now >= self.expires_at {
            self.status = ConsentStatus::Expired;
            return Err(ConsentError::Expired);
        }
        self.status = ConsentStatus::Declined;
        Ok(self.status)
    }

    /// Revoke pending or mutual consent before settlement.
    ///
    /// Revocation is available to either participant and does not depend on
    /// reputation, contribution, gratitude or a justification narrative.
    pub fn revoke(
        &mut self,
        actor: &AuthenticatedSubject,
        digest: &ProposalDigest,
    ) -> Result<ConsentStatus, ConsentError> {
        self.validate_actor_and_digest(actor, digest)?;
        if self.settled {
            return Err(ConsentError::AlreadySettled);
        }
        if self.status == ConsentStatus::Revoked {
            return Ok(ConsentStatus::Revoked);
        }
        if matches!(self.status, ConsentStatus::Declined | ConsentStatus::Expired) {
            return Err(ConsentError::Revoked);
        }
        self.status = ConsentStatus::Revoked;
        Ok(self.status)
    }

    /// Mark the proposal settled after both acceptances.
    ///
    /// The host adapter must additionally authorize this transition and make
    /// external settlement idempotent. This method only checks the model state.
    pub fn mark_settled(&mut self) -> Result<(), ConsentError> {
        if self.status != ConsentStatus::Accepted {
            return Err(ConsentError::NotAccepted);
        }
        if self.settled {
            return Ok(());
        }
        self.settled = true;
        Ok(())
    }

    fn party_index(&self, actor: &AuthenticatedSubject) -> Option<usize> {
        self.parties.iter().position(|party| party == actor)
    }

    fn validate_actor_and_digest(
        &self,
        actor: &AuthenticatedSubject,
        digest: &ProposalDigest,
    ) -> Result<(), ConsentError> {
        if digest != &self.digest {
            return Err(ConsentError::ProposalMismatch);
        }
        if self.party_index(actor).is_none() {
            return Err(ConsentError::NotParticipant);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(name: &str) -> AuthenticatedSubject {
        AuthenticatedSubject::from_verified_adapter(name).unwrap()
    }

    fn pair() -> (BilateralConsent, AuthenticatedSubject, AuthenticatedSubject, ProposalDigest) {
        let a = subject("requester");
        let b = subject("provider");
        let digest = [0x5A; 32];
        let pair = BilateralConsent::new(digest, a.clone(), b.clone(), 100, 200).unwrap();
        (pair, a, b, digest)
    }

    #[test]
    fn one_party_acceptance_does_not_enable_settlement() {
        let (mut pair, a, _, digest) = pair();
        assert_eq!(pair.accept(&a, &digest, 101), Ok(ConsentStatus::Pending));
        assert_eq!(pair.status(), ConsentStatus::Pending);
        assert!(!pair.can_settle());
    }

    #[test]
    fn both_distinct_parties_must_accept_the_same_proposal() {
        let (mut pair, a, b, digest) = pair();
        assert_eq!(pair.accept(&a, &digest, 101), Ok(ConsentStatus::Pending));
        assert_eq!(pair.accept(&b, &digest, 102), Ok(ConsentStatus::Accepted));
        assert_eq!(pair.status(), ConsentStatus::Accepted);
        assert!(pair.can_settle());
    }

    #[test]
    fn duplicate_acceptance_is_idempotent() {
        let (mut pair, a, b, digest) = pair();
        pair.accept(&a, &digest, 101).unwrap();
        assert_eq!(pair.accept(&a, &digest, 102), Ok(ConsentStatus::Pending));
        pair.accept(&b, &digest, 103).unwrap();
        assert_eq!(pair.accept(&b, &digest, 104), Ok(ConsentStatus::Accepted));
        assert!(pair.can_settle());
    }

    #[test]
    fn outsider_cannot_accept_or_decline() {
        let (mut pair, _, _, digest) = pair();
        let outsider = subject("outsider");
        let before = pair.clone();
        assert_eq!(
            pair.accept(&outsider, &digest, 101),
            Err(ConsentError::NotParticipant)
        );
        assert_eq!(
            pair.decline(&outsider, &digest, 101),
            Err(ConsentError::NotParticipant)
        );
        assert_eq!(pair, before);
    }

    #[test]
    fn changed_terms_require_new_consent() {
        let (mut pair, a, _, digest) = pair();
        let changed = [0xA5; 32];
        let before = pair.clone();
        assert_eq!(
            pair.accept(&a, &changed, 101),
            Err(ConsentError::ProposalMismatch)
        );
        assert_eq!(pair, before);
    }

    #[test]
    fn decline_is_terminal_and_does_not_require_a_reason() {
        let (mut pair, a, b, digest) = pair();
        assert_eq!(pair.decline(&a, &digest, 101), Ok(ConsentStatus::Declined));
        assert_eq!(
            pair.accept(&b, &digest, 102),
            Err(ConsentError::AlreadyDeclined)
        );
        assert!(!pair.can_settle());
    }

    #[test]
    fn either_party_can_revoke_before_settlement() {
        let (mut pair, a, b, digest) = pair();
        pair.accept(&a, &digest, 101).unwrap();
        pair.accept(&b, &digest, 102).unwrap();
        assert_eq!(pair.revoke(&a, &digest), Ok(ConsentStatus::Revoked));
        assert!(!pair.can_settle());
    }

    #[test]
    fn settlement_requires_both_acceptances_and_is_idempotent() {
        let (mut pair, a, b, digest) = pair();
        assert_eq!(pair.mark_settled(), Err(ConsentError::NotAccepted));
        pair.accept(&a, &digest, 101).unwrap();
        pair.accept(&b, &digest, 102).unwrap();
        pair.mark_settled().unwrap();
        pair.mark_settled().unwrap();
        assert!(pair.is_settled());
        assert!(!pair.can_settle());
        assert_eq!(pair.revoke(&a, &digest), Err(ConsentError::AlreadySettled));
    }

    #[test]
    fn expiry_prevents_late_acceptance() {
        let (mut pair, a, _, digest) = pair();
        assert_eq!(pair.accept(&a, &digest, 200), Err(ConsentError::Expired));
        assert_eq!(pair.status(), ConsentStatus::Expired);
        assert!(!pair.can_settle());
    }

    #[test]
    fn same_party_cannot_fill_both_roles() {
        let a = subject("same-agent");
        assert_eq!(
            BilateralConsent::new([0; 32], a.clone(), a, 10, 20),
            Err(ConsentError::SameParty)
        );
    }
}
