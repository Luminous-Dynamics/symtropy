#![forbid(unsafe_code)]
//! Synchronous admission gate for asynchronous wearer-governed adaptation proposals.
//! This checks freshness and supersession only; it is not a controller or safety authority.

use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProposalClock { pub clock_id: u64, pub boot_epoch: u64 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdaptationProposal {
    pub proposal_id: u64,
    pub proposal_sequence: u64,
    pub agency_epoch: u64,
    pub clock: ProposalClock,
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub state_revision_digest: [u8; 32],
    pub source_observation_sequence: u64,
    pub source_observation_timestamp_ns: u64,
    pub maximum_observation_age_ns: u64,
    pub parameter_digest: [u8; 32],
    pub optimizer_receipt_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionContext {
    pub clock: ProposalClock,
    pub now_ns: u64,
    /// Must be advanced by a trusted proposal sequencer when a newer proposal is issued.
    pub latest_issued_proposal_sequence: u64,
    pub latest_observation_sequence: u64,
    pub current_state_revision_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProposalAdmissionReceipt {
    proposal_id: u64,
    proposal_sequence: u64,
    agency_epoch: u64,
    clock: ProposalClock,
    expires_at_ns: u64,
    state_revision_digest: [u8; 32],
    source_observation_sequence: u64,
    source_observation_timestamp_ns: u64,
    maximum_observation_age_ns: u64,
    parameter_digest: [u8; 32],
    optimizer_receipt_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalRejectReason {
    ZeroProposalId, ZeroProposalSequence, ZeroAgencyEpoch, ZeroClockIdentity,
    MissingStateRevisionDigest, MissingParameterDigest, MissingOptimizerReceiptDigest,
    InvalidExpiry, InvalidObservationAge, ClockMismatch, AgencyEpochMismatch,
    ProposalNotLatestIssued, ProposalAlreadyConsumed, ProposalIssuedInFuture,
    ProposalExpired, StateRevisionMismatch, ObservationFromFuture, ObservationStale,
    SequenceOverflow,
}
impl fmt::Display for ProposalRejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for ProposalRejectReason {}

/// Single-owner fast-loop gate. Caller owns thread synchronization and stop-path integration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProposalAdmissionGate { agency_epoch: u64, last_admitted_sequence: u64 }

impl ProposalAdmissionGate {
    pub const fn new(initial_agency_epoch: u64) -> Self {
        Self { agency_epoch: initial_agency_epoch, last_admitted_sequence: 0 }
    }

    /// Revoke every proposal from the previous wearer-agency epoch.
    pub fn revoke_all(&mut self) -> Result<u64, ProposalRejectReason> {
        let next = self.agency_epoch.checked_add(1).ok_or(ProposalRejectReason::SequenceOverflow)?;
        self.agency_epoch = next;
        Ok(next)
    }

    /// Admit a current proposal. Rejected proposals do not mutate gate state.
    pub fn admit(
        &mut self,
        proposal: AdaptationProposal,
        context: AdmissionContext,
    ) -> Result<ProposalAdmissionReceipt, ProposalRejectReason> {
        validate_proposal(proposal)?;
        validate_context(context)?;
        if proposal.clock != context.clock { return Err(ProposalRejectReason::ClockMismatch); }
        if proposal.agency_epoch != self.agency_epoch { return Err(ProposalRejectReason::AgencyEpochMismatch); }
        if proposal.proposal_sequence != context.latest_issued_proposal_sequence {
            return Err(ProposalRejectReason::ProposalNotLatestIssued);
        }
        if proposal.proposal_sequence <= self.last_admitted_sequence {
            return Err(ProposalRejectReason::ProposalAlreadyConsumed);
        }
        if proposal.issued_at_ns > context.now_ns { return Err(ProposalRejectReason::ProposalIssuedInFuture); }
        if context.now_ns >= proposal.expires_at_ns { return Err(ProposalRejectReason::ProposalExpired); }
        if proposal.source_observation_sequence == 0 { return Err(ProposalRejectReason::ZeroObservationSequence); }
        if proposal.source_observation_sequence > context.latest_observation_sequence {
            return Err(ProposalRejectReason::ObservationSequenceFromFuture);
        }
        if proposal.state_revision_digest != context.current_state_revision_digest {
            return Err(ProposalRejectReason::StateRevisionMismatch);
        }
        validate_observation_age(proposal.source_observation_timestamp_ns,
            proposal.maximum_observation_age_ns, context.now_ns)?;

        let receipt = ProposalAdmissionReceipt {
            proposal_id: proposal.proposal_id,
            proposal_sequence: proposal.proposal_sequence,
            agency_epoch: proposal.agency_epoch,
            clock: proposal.clock,
            expires_at_ns: proposal.expires_at_ns,
            state_revision_digest: proposal.state_revision_digest,
            source_observation_sequence: proposal.source_observation_sequence,
            source_observation_timestamp_ns: proposal.source_observation_timestamp_ns,
            maximum_observation_age_ns: proposal.maximum_observation_age_ns,
            parameter_digest: proposal.parameter_digest,
            optimizer_receipt_digest: proposal.optimizer_receipt_digest,
        };
        self.last_admitted_sequence = proposal.proposal_sequence;
        Ok(receipt)
    }

    /// Revalidate immediately before parameter application; this still grants no actuator authority.
    pub fn revalidate_for_application(
        &self,
        receipt: ProposalAdmissionReceipt,
        context: AdmissionContext,
    ) -> Result<(), ProposalRejectReason> {
        validate_context(context)?;
        if receipt.clock != context.clock { return Err(ProposalRejectReason::ClockMismatch); }
        if receipt.agency_epoch != self.agency_epoch { return Err(ProposalRejectReason::AgencyEpochMismatch); }
        if receipt.proposal_sequence != context.latest_issued_proposal_sequence {
            return Err(ProposalRejectReason::ProposalNotLatestIssued);
        }
        if receipt.proposal_sequence != self.last_admitted_sequence {
            return Err(ProposalRejectReason::ProposalAlreadyConsumed);
        }
        if context.now_ns >= receipt.expires_at_ns { return Err(ProposalRejectReason::ProposalExpired); }
        if receipt.source_observation_sequence == 0 { return Err(ProposalRejectReason::ZeroObservationSequence); }
        if receipt.source_observation_sequence > context.latest_observation_sequence {
            return Err(ProposalRejectReason::ObservationSequenceFromFuture);
        }
        if receipt.state_revision_digest != context.current_state_revision_digest {
            return Err(ProposalRejectReason::StateRevisionMismatch);
        }
        validate_observation_age(receipt.source_observation_timestamp_ns,
            receipt.maximum_observation_age_ns, context.now_ns)
    }

    /// Read-only proposal identifier and result metadata for downstream receipts.
    pub const fn proposal_id(&self) -> u64 { self.proposal_id }
    pub const fn proposal_sequence(&self) -> u64 { self.proposal_sequence }
    pub const fn agency_epoch_id(&self) -> u64 { self.agency_epoch }
    pub const fn expires_at_ns(&self) -> u64 { self.expires_at_ns }
    pub const fn parameter_digest(&self) -> &[u8; 32] { &self.parameter_digest }
    pub const fn optimizer_receipt_digest(&self) -> &[u8; 32] { &self.optimizer_receipt_digest }

    pub const fn agency_epoch(&self) -> u64 { self.agency_epoch }
    pub const fn last_admitted_sequence(&self) -> u64 { self.last_admitted_sequence }
}

fn validate_proposal(p: AdaptationProposal) -> Result<(), ProposalRejectReason> {
    if p.proposal_id == 0 { return Err(ProposalRejectReason::ZeroProposalId); }
    if p.proposal_sequence == 0 { return Err(ProposalRejectReason::ZeroProposalSequence); }
    if p.agency_epoch == 0 { return Err(ProposalRejectReason::ZeroAgencyEpoch); }
    if p.clock.clock_id == 0 || p.clock.boot_epoch == 0 { return Err(ProposalRejectReason::ZeroClockIdentity); }
    if p.state_revision_digest == [0; 32] { return Err(ProposalRejectReason::MissingStateRevisionDigest); }
    if p.parameter_digest == [0; 32] { return Err(ProposalRejectReason::MissingParameterDigest); }
    if p.optimizer_receipt_digest == [0; 32] { return Err(ProposalRejectReason::MissingOptimizerReceiptDigest); }
    if p.expires_at_ns <= p.issued_at_ns { return Err(ProposalRejectReason::InvalidExpiry); }
    if p.maximum_observation_age_ns == 0 { return Err(ProposalRejectReason::InvalidObservationAge); }
    Ok(())
}
fn validate_context(c: AdmissionContext) -> Result<(), ProposalRejectReason> {
    if c.clock.clock_id == 0 || c.clock.boot_epoch == 0 { return Err(ProposalRejectReason::ZeroClockIdentity); }
    if c.current_state_revision_digest == [0; 32] { return Err(ProposalRejectReason::MissingStateRevisionDigest); }
    if c.latest_observation_sequence == 0 { return Err(ProposalRejectReason::ZeroObservationSequence); }
    Ok(())
}
fn validate_observation_age(timestamp: u64, maximum: u64, now: u64) -> Result<(), ProposalRejectReason> {
    if timestamp > now { return Err(ProposalRejectReason::ObservationFromFuture); }
    if now - timestamp > maximum { return Err(ProposalRejectReason::ObservationStale); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const STATE: [u8;32] = [0x11;32];
    const PARAM: [u8;32] = [0x22;32];
    const OPT: [u8;32] = [0x33;32];
    fn clock() -> ProposalClock { ProposalClock { clock_id: 7, boot_epoch: 9 } }
    fn context(seq:u64) -> AdmissionContext {
        AdmissionContext { clock:clock(), now_ns:1000, latest_issued_proposal_sequence:seq,
            current_state_revision_digest:STATE }
    }
    fn proposal(seq:u64) -> AdaptationProposal {
        AdaptationProposal { proposal_id:seq+100, proposal_sequence:seq, agency_epoch:1,
            clock:clock(), issued_at_ns:900, expires_at_ns:1100, state_revision_digest:STATE,
            source_observation_sequence:42, source_observation_timestamp_ns:950,
            maximum_observation_age_ns:100, parameter_digest:PARAM, optimizer_receipt_digest:OPT }
    }

    #[test] fn current_proposal_is_admitted_and_revalidated() {
        let mut gate=ProposalAdmissionGate::new(1);
        let receipt=gate.admit(proposal(1),context(1)).unwrap();
        assert_eq!(gate.last_admitted_sequence(),1);
        assert_eq!(gate.revalidate_for_application(receipt,context(1)),Ok(()));
    }
    #[test] fn late_old_result_is_rejected_after_newer_proposal_is_issued() {
        let mut gate=ProposalAdmissionGate::new(1);
        assert_eq!(gate.admit(proposal(1),context(2)),Err(ProposalRejectReason::ProposalNotLatestIssued));
        assert_eq!(gate.last_admitted_sequence(),0);
        assert!(gate.admit(proposal(2),context(2)).is_ok());
    }
    #[test] fn expired_and_future_issued_proposals_are_rejected() {
        let mut gate=ProposalAdmissionGate::new(1);
        let mut p=proposal(1); p.expires_at_ns=1000;
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::ProposalExpired));
        p=proposal(1); p.issued_at_ns=1001; p.expires_at_ns=1200;
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::ProposalIssuedInFuture));
        assert_eq!(gate.last_admitted_sequence(),0);
    }
    #[test] fn source_observation_sequence_from_future_is_rejected() {
        let mut gate=ProposalAdmissionGate::new(1);
        let mut p=proposal(1); p.source_observation_sequence=43;
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::ObservationSequenceFromFuture));
        assert_eq!(gate.last_admitted_sequence(),0);
    }
    #[test] fn changed_state_revision_rejects_proposal() {
        let mut gate=ProposalAdmissionGate::new(1);
        let mut c=context(1); c.current_state_revision_digest[0]^=1;
        assert_eq!(gate.admit(proposal(1),c),Err(ProposalRejectReason::StateRevisionMismatch));
    }
    #[test] fn stale_and_future_source_observations_are_rejected() {
        let mut gate=ProposalAdmissionGate::new(1);
        let mut p=proposal(1); p.source_observation_timestamp_ns=899;
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::ObservationStale));
        p=proposal(1); p.source_observation_timestamp_ns=1001;
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::ObservationFromFuture));
    }
    #[test] fn clock_and_boot_epoch_mismatches_are_rejected() {
        let mut gate=ProposalAdmissionGate::new(1);
        let mut p=proposal(1); p.clock.clock_id+=1;
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::ClockMismatch));
        p=proposal(1); p.clock.boot_epoch+=1;
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::ClockMismatch));
    }
    #[test] fn replay_and_duplicate_sequence_are_rejected() {
        let mut gate=ProposalAdmissionGate::new(1); let p=proposal(1);
        gate.admit(p,context(1)).unwrap();
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::ProposalAlreadyConsumed));
    }
    #[test] fn wearer_revocation_invalidates_pending_proposals_and_receipts() {
        let mut gate=ProposalAdmissionGate::new(1);
        let receipt=gate.admit(proposal(1),context(1)).unwrap();
        assert_eq!(gate.revoke_all(),Ok(2));
        assert_eq!(gate.revalidate_for_application(receipt,context(1)),Err(ProposalRejectReason::AgencyEpochMismatch));
        assert_eq!(gate.admit(proposal(2),context(2)),Err(ProposalRejectReason::AgencyEpochMismatch));
        let mut fresh=proposal(2); fresh.agency_epoch=2;
        assert!(gate.admit(fresh,context(2)).is_ok());
    }
    #[test] fn newer_proposal_invalidates_previous_receipt_at_application() {
        let mut gate=ProposalAdmissionGate::new(1);
        let receipt=gate.admit(proposal(1),context(1)).unwrap();
        assert_eq!(gate.revalidate_for_application(receipt,context(2)),Err(ProposalRejectReason::ProposalNotLatestIssued));
    }
    #[test] fn application_revalidation_checks_expiry_and_state_revision() {
        let mut gate=ProposalAdmissionGate::new(1);
        let receipt=gate.admit(proposal(1),context(1)).unwrap();
        let mut c=context(1); c.now_ns=receipt.expires_at_ns;
        assert_eq!(gate.revalidate_for_application(receipt,c),Err(ProposalRejectReason::ProposalExpired));
        c=context(1); c.current_state_revision_digest[0]^=0x80;
        assert_eq!(gate.revalidate_for_application(receipt,c),Err(ProposalRejectReason::StateRevisionMismatch));
    }
    #[test] fn invalid_identity_and_missing_digests_are_rejected() {
        let mut gate=ProposalAdmissionGate::new(1);
        let mut p=proposal(1); p.proposal_id=0;
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::ZeroProposalId));
        p=proposal(1); p.optimizer_receipt_digest=[0;32];
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::MissingOptimizerReceiptDigest));
        assert_eq!(gate.last_admitted_sequence(),0);
    }
    #[test] fn rejection_does_not_mutate_gate_state() {
        let mut gate=ProposalAdmissionGate::new(1);
        let mut p=proposal(1); p.parameter_digest=[0;32];
        assert_eq!(gate.admit(p,context(1)),Err(ProposalRejectReason::MissingParameterDigest));
        assert_eq!(gate.last_admitted_sequence(),0);
        assert!(gate.admit(proposal(1),context(1)).is_ok());
    }
    #[test] fn observation_freshness_is_rechecked_at_application() {
        let mut gate=ProposalAdmissionGate::new(1);
        let receipt=gate.admit(proposal(1),context(1)).unwrap();
        let mut c=context(1); c.now_ns=1051;
        assert_eq!(gate.revalidate_for_application(receipt,c),Err(ProposalRejectReason::ObservationStale));
    }
}
