#![forbid(unsafe_code)]
//! Conjunctive, identity-bound pHRI admission reference.
//!
//! This crate evaluates caller-qualified evidence. It is not a sensor,
//! actuator, real-time scheduler, physical stop, or safety certification.

use core::fmt;

pub type Digest = [u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockIdentity {
    pub clock_id: u64,
    pub boot_epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceIdentity {
    pub sequence: u64,
    pub proposal_sequence: u64,
    pub sampled_at_ns: u64,
    pub proposal_expires_at_ns: u64,
    pub clock: ClockIdentity,
    pub interaction_epoch: u64,
    pub state_revision_digest: Digest,
    pub profile_digest: Digest,
    pub command_digest: Digest,
    pub port_graph_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionEnvelopeLimits {
    pub max_sample_age_ns: u64,
    pub max_sample_gap_ns: u64,
    pub max_energy_residual_upper_j: f64,
    pub max_energy_prefix_deficit_upper_j: f64,
    pub max_local_pressure_upper_pa: f64,
    pub max_pressure_time_dose_upper_pa_s: f64,
    pub max_deformation_rate_upper_s_inv: f64,
    pub max_contact_power_upper_w: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionEnvelopeProfile {
    /// Digest of the exact profile and all thresholds used by upstream checks.
    pub digest: Digest,
    pub limits: InteractionEnvelopeLimits,
}

impl InteractionEnvelopeProfile {
    fn validate(self) -> Result<(), RejectReason> {
        let l = self.limits;
        if self.digest == [0; 32] {
            return Err(RejectReason::MissingProfileDigest);
        }
        if l.max_sample_age_ns == 0 || l.max_sample_gap_ns == 0 {
            return Err(RejectReason::InvalidProfile);
        }
        let limits = [
            l.max_energy_residual_upper_j,
            l.max_energy_prefix_deficit_upper_j,
            l.max_local_pressure_upper_pa,
            l.max_pressure_time_dose_upper_pa_s,
            l.max_deformation_rate_upper_s_inv,
            l.max_contact_power_upper_w,
        ];
        if !limits.iter().all(|value| finite_nonnegative(*value)) {
            return Err(RejectReason::InvalidProfile);
        }
        Ok(())
    }
}

/// Conservative upper bounds produced by an upstream energy observer.
/// boundary_complete is a caller assertion about the selected control
/// volume; this crate does not establish physical boundary completeness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnergyEvidence {
    pub identity: EvidenceIdentity,
    pub residual_upper_j: f64,
    pub maximum_prefix_deficit_upper_j: f64,
    pub boundary_complete: bool,
    pub measurement_digest: Digest,
}

/// Local contact-exposure upper bounds from a caller-qualified measurement
/// pipeline. The deformation value is a proxy, not a tissue injury metric.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactEvidence {
    pub identity: EvidenceIdentity,
    pub local_pressure_upper_pa: f64,
    pub pressure_time_dose_upper_pa_s: f64,
    pub deformation_rate_upper_s_inv: f64,
    pub contact_power_upper_w: f64,
    pub calibration_digest: Digest,
    pub sensor_profile_digest: Digest,
    pub measurement_digest: Digest,
}

/// Explicit wearer authority. No tactile/pressure signal is interpreted as
/// consent or as permission to continue an interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgencyEvidence {
    pub identity: EvidenceIdentity,
    pub agency_epoch: u64,
    pub explicit_user_authorization_active: bool,
    pub stop_latched: bool,
    pub authorization_digest: Digest,
}

/// A separate authority channel. This record is an upstream claim, not a
/// physical actuator interlock implemented by this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActuatorAuthorityEvidence {
    pub identity: EvidenceIdentity,
    pub authority_epoch: u64,
    pub permit_active: bool,
    pub authority_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvidenceBundle {
    pub energy: EnergyEvidence,
    pub contact: ContactEvidence,
    pub agency: AgencyEvidence,
    pub actuator: ActuatorAuthorityEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionContext {
    pub now_ns: u64,
    pub clock: ClockIdentity,
    /// Must come from a trusted proposal sequencer, not from the proposal.
    pub latest_issued_proposal_sequence: u64,
    pub current_state_revision_digest: Digest,
    pub current_command_digest: Digest,
    pub current_port_graph_digest: Digest,
    /// Current epoch supplied by the independent actuator-authority service.
    pub current_actuator_authority_epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectReason {
    MissingProfileDigest,
    InvalidProfile,
    ZeroInteractionSequence,
    ZeroProposalSequence,
    ZeroClockIdentity,
    ZeroInteractionEpoch,
    MissingStateRevisionDigest,
    MissingCommandDigest,
    MissingPortGraphDigest,
    MissingEvidenceDigest,
    MissingCalibrationDigest,
    MissingSensorProfileDigest,
    MissingAuthorizationDigest,
    MissingActuatorAuthorityDigest,
    InvalidProposalExpiry,
    EvidenceIdentityMismatch,
    ClockMismatch,
    ProfileMismatch,
    StateRevisionMismatch,
    CommandMismatch,
    PortGraphMismatch,
    ProposalNotLatestIssued,
    ProposalExpired,
    ObservationFromFuture,
    ObservationStale,
    NonIncreasingSampleTime,
    SampleGapExceeded,
    InteractionSequenceReplay,
    AgencyEpochMismatch,
    ExplicitUserAuthorizationAbsent,
    StopLatched,
    EnergyBoundaryIncomplete,
    InvalidEnergyEvidence,
    EnergyResidualExceeded,
    EnergyPrefixDeficitExceeded,
    InvalidContactEvidence,
    LocalPressureExceeded,
    PressureTimeDoseExceeded,
    DeformationRateExceeded,
    ContactPowerExceeded,
    ActuatorAuthorityAbsent,
    ActuatorAuthorityEpochMismatch,
    ReceiptNotLatest,
    ReceiptAlreadyConsumed,
    ReceiptEvidenceMismatch,
    SequenceOverflow,
    ExplicitRearmRequired,
    RearmDigestMissing,
    RearmEpochMismatch,
}

impl fmt::Display for RejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RejectReason {}

/// Opaque, one-use admission receipt. It binds every evidence digest to one
/// command and one interaction state. It is not a cryptographic signature.
#[derive(Debug, PartialEq, Eq)]
pub struct InteractionAdmissionReceipt {
    receipt_id: u64,
    identity: EvidenceIdentity,
    evidence_digests: [Digest; 4],
}

impl InteractionAdmissionReceipt {
    pub const fn sequence(&self) -> u64 {
        self.identity.sequence
    }

    pub const fn command_digest(&self) -> &Digest {
        &self.identity.command_digest
    }

    pub const fn profile_digest(&self) -> &Digest {
        &self.identity.profile_digest
    }
}

/// Result of successful pre-application revalidation. This is a software
/// admission token only; the caller still owns the physical actuator boundary.
#[derive(Debug, PartialEq, Eq)]
pub struct ValidatedApplication {
    receipt_id: u64,
    sequence: u64,
    command_digest: Digest,
}

impl ValidatedApplication {
    pub const fn receipt_id(&self) -> u64 {
        self.receipt_id
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn command_digest(&self) -> &Digest {
        &self.command_digest
    }
}

/// Single-owner state machine. The caller is responsible for synchronization.
#[derive(Debug)]
pub struct InteractionEnvelopeGate {
    profile: InteractionEnvelopeProfile,
    agency_epoch: u64,
    stop_latched: bool,
    last_admitted_sequence: u64,
    last_sample_time_ns: Option<u64>,
    next_receipt_id: u64,
    last_admitted_receipt_id: Option<u64>,
    last_consumed_receipt_id: Option<u64>,
}

impl InteractionEnvelopeGate {
    pub fn new(
        profile: InteractionEnvelopeProfile,
        initial_agency_epoch: u64,
    ) -> Result<Self, RejectReason> {
        profile.validate()?;
        if initial_agency_epoch == 0 {
            return Err(RejectReason::ZeroInteractionEpoch);
        }
        Ok(Self {
            profile,
            agency_epoch: initial_agency_epoch,
            stop_latched: false,
            last_admitted_sequence: 0,
            last_sample_time_ns: None,
            next_receipt_id: 1,
            last_admitted_receipt_id: None,
            last_consumed_receipt_id: None,
        })
    }

    /// Revoke all outstanding admissions. This latches the software gate and
    /// advances the wearer epoch; it does not actuate the physical stop.
    pub fn revoke_all(&mut self) -> Result<u64, RejectReason> {
        let next_epoch = self
            .agency_epoch
            .checked_add(1)
            .ok_or(RejectReason::SequenceOverflow)?;
        self.agency_epoch = next_epoch;
        self.stop_latched = true;
        self.last_admitted_receipt_id = None;
        Ok(next_epoch)
    }

    /// Clear the software latch only after a caller-observed explicit user
    /// re-arm action bound to the current epoch. Sensors cannot invoke this
    /// on the user's behalf.
    pub fn rearm_after_explicit_user_action(
        &mut self,
        observed_agency_epoch: u64,
        user_action_digest: Digest,
    ) -> Result<(), RejectReason> {
        if !self.stop_latched {
            return Err(RejectReason::ExplicitRearmRequired);
        }
        if observed_agency_epoch != self.agency_epoch {
            return Err(RejectReason::RearmEpochMismatch);
        }
        if user_action_digest == [0; 32] {
            return Err(RejectReason::RearmDigestMissing);
        }
        self.stop_latched = false;
        Ok(())
    }

    /// Admit only if every evidence domain qualifies against the same event.
    /// Rejection never mutates the gate's sequence or receipt state.
    pub fn admit(
        &mut self,
        bundle: EvidenceBundle,
        context: AdmissionContext,
    ) -> Result<InteractionAdmissionReceipt, Vec<RejectReason>> {
        let mut reasons = self.evaluate_bundle(bundle, context);
        let identity = bundle.energy.identity;
        if identity.sequence <= self.last_admitted_sequence {
            reasons.push(RejectReason::InteractionSequenceReplay);
        }
        if let Some(previous_time) = self.last_sample_time_ns {
            if identity.sampled_at_ns <= previous_time {
                reasons.push(RejectReason::NonIncreasingSampleTime);
            } else if identity.sampled_at_ns - previous_time
                > self.profile.limits.max_sample_gap_ns
            {
                reasons.push(RejectReason::SampleGapExceeded);
            }
        }
        if !reasons.is_empty() {
            return Err(reasons);
        }

        let next_receipt_id = self
            .next_receipt_id
            .checked_add(1)
            .ok_or_else(|| vec![RejectReason::SequenceOverflow])?;
        let receipt = InteractionAdmissionReceipt {
            receipt_id: self.next_receipt_id,
            identity,
            evidence_digests: evidence_digests(bundle),
        };

        self.last_admitted_sequence = identity.sequence;
        self.last_sample_time_ns = Some(identity.sampled_at_ns);
        self.last_admitted_receipt_id = Some(receipt.receipt_id);
        self.next_receipt_id = next_receipt_id;
        Ok(receipt)
    }

    /// Re-evaluate the entire bundle immediately before the caller's
    /// application boundary. A newer admission, stop, stale sample, state
    /// change, authority change, or changed evidence digest invalidates it.
    pub fn revalidate_for_application(
        &mut self,
        receipt: InteractionAdmissionReceipt,
        bundle: EvidenceBundle,
        context: AdmissionContext,
    ) -> Result<ValidatedApplication, Vec<RejectReason>> {
        let mut reasons = self.evaluate_bundle(bundle, context);
        if self.stop_latched {
            reasons.push(RejectReason::StopLatched);
        }
        if self.last_admitted_receipt_id != Some(receipt.receipt_id) {
            reasons.push(RejectReason::ReceiptNotLatest);
        }
        if self.last_consumed_receipt_id == Some(receipt.receipt_id) {
            reasons.push(RejectReason::ReceiptAlreadyConsumed);
        }
        if bundle.energy.identity != receipt.identity
            || evidence_digests(bundle) != receipt.evidence_digests
        {
            reasons.push(RejectReason::ReceiptEvidenceMismatch);
        }
        if !reasons.is_empty() {
            return Err(reasons);
        }

        self.last_consumed_receipt_id = Some(receipt.receipt_id);
        Ok(ValidatedApplication {
            receipt_id: receipt.receipt_id,
            sequence: receipt.identity.sequence,
            command_digest: receipt.identity.command_digest,
        })
    }

    pub const fn agency_epoch(&self) -> u64 {
        self.agency_epoch
    }

    pub const fn stop_latched(&self) -> bool {
        self.stop_latched
    }

    pub const fn last_admitted_sequence(&self) -> u64 {
        self.last_admitted_sequence
    }

    fn evaluate_bundle(
        &self,
        bundle: EvidenceBundle,
        context: AdmissionContext,
    ) -> Vec<RejectReason> {
        let mut reasons = Vec::new();
        let identity = bundle.energy.identity;
        let profile = self.profile;
        let limits = profile.limits;

        if identity.sequence == 0 {
            reasons.push(RejectReason::ZeroInteractionSequence);
        }
        if identity.proposal_sequence == 0 {
            reasons.push(RejectReason::ZeroProposalSequence);
        }
        if identity.clock.clock_id == 0 || identity.clock.boot_epoch == 0 {
            reasons.push(RejectReason::ZeroClockIdentity);
        }
        if identity.interaction_epoch == 0 {
            reasons.push(RejectReason::ZeroInteractionEpoch);
        }
        if identity.state_revision_digest == [0; 32] {
            reasons.push(RejectReason::MissingStateRevisionDigest);
        }
        if identity.command_digest == [0; 32] {
            reasons.push(RejectReason::MissingCommandDigest);
        }
        if identity.port_graph_digest == [0; 32] {
            reasons.push(RejectReason::MissingPortGraphDigest);
        }
        if identity.proposal_expires_at_ns <= identity.sampled_at_ns {
            reasons.push(RejectReason::InvalidProposalExpiry);
        }
        if identity.clock != context.clock {
            reasons.push(RejectReason::ClockMismatch);
        }
        if identity.profile_digest != profile.digest {
            reasons.push(RejectReason::ProfileMismatch);
        }
        if identity.state_revision_digest != context.current_state_revision_digest {
            reasons.push(RejectReason::StateRevisionMismatch);
        }
        if identity.command_digest != context.current_command_digest {
            reasons.push(RejectReason::CommandMismatch);
        }
        if identity.port_graph_digest != context.current_port_graph_digest {
            reasons.push(RejectReason::PortGraphMismatch);
        }
        if identity.proposal_sequence != context.latest_issued_proposal_sequence {
            reasons.push(RejectReason::ProposalNotLatestIssued);
        }
        if context.now_ns >= identity.proposal_expires_at_ns {
            reasons.push(RejectReason::ProposalExpired);
        }
        if identity.sampled_at_ns > context.now_ns {
            reasons.push(RejectReason::ObservationFromFuture);
        } else if context.now_ns - identity.sampled_at_ns > limits.max_sample_age_ns {
            reasons.push(RejectReason::ObservationStale);
        }

        if bundle.contact.identity != identity
            || bundle.agency.identity != identity
            || bundle.actuator.identity != identity
        {
            reasons.push(RejectReason::EvidenceIdentityMismatch);
        }

        if bundle.energy.measurement_digest == [0; 32]
            || bundle.contact.measurement_digest == [0; 32]
        {
            reasons.push(RejectReason::MissingEvidenceDigest);
        }
        if bundle.contact.calibration_digest == [0; 32] {
            reasons.push(RejectReason::MissingCalibrationDigest);
        }
        if bundle.contact.sensor_profile_digest == [0; 32] {
            reasons.push(RejectReason::MissingSensorProfileDigest);
        }
        if bundle.agency.authorization_digest == [0; 32] {
            reasons.push(RejectReason::MissingAuthorizationDigest);
        }
        if bundle.actuator.authority_digest == [0; 32] {
            reasons.push(RejectReason::MissingActuatorAuthorityDigest);
        }

        if bundle.agency.agency_epoch != self.agency_epoch
            || identity.interaction_epoch != self.agency_epoch
        {
            reasons.push(RejectReason::AgencyEpochMismatch);
        }
        if self.stop_latched || bundle.agency.stop_latched {
            reasons.push(RejectReason::StopLatched);
        }
        if !bundle.agency.explicit_user_authorization_active {
            reasons.push(RejectReason::ExplicitUserAuthorizationAbsent);
        }

        if !bundle.energy.boundary_complete {
            reasons.push(RejectReason::EnergyBoundaryIncomplete);
        }
        if !bundle.energy.residual_upper_j.is_finite()
            || !finite_nonnegative(bundle.energy.maximum_prefix_deficit_upper_j)
        {
            reasons.push(RejectReason::InvalidEnergyEvidence);
        } else {
            if bundle.energy.residual_upper_j > limits.max_energy_residual_upper_j {
                reasons.push(RejectReason::EnergyResidualExceeded);
            }
            if bundle.energy.maximum_prefix_deficit_upper_j
                > limits.max_energy_prefix_deficit_upper_j
            {
                reasons.push(RejectReason::EnergyPrefixDeficitExceeded);
            }
        }

        let contact_values = [
            bundle.contact.local_pressure_upper_pa,
            bundle.contact.pressure_time_dose_upper_pa_s,
            bundle.contact.deformation_rate_upper_s_inv,
            bundle.contact.contact_power_upper_w,
        ];
        if !contact_values
            .iter()
            .all(|value| finite_nonnegative(*value))
        {
            reasons.push(RejectReason::InvalidContactEvidence);
        } else {
            if bundle.contact.local_pressure_upper_pa > limits.max_local_pressure_upper_pa {
                reasons.push(RejectReason::LocalPressureExceeded);
            }
            if bundle.contact.pressure_time_dose_upper_pa_s
                > limits.max_pressure_time_dose_upper_pa_s
            {
                reasons.push(RejectReason::PressureTimeDoseExceeded);
            }
            if bundle.contact.deformation_rate_upper_s_inv
                > limits.max_deformation_rate_upper_s_inv
            {
                reasons.push(RejectReason::DeformationRateExceeded);
            }
            if bundle.contact.contact_power_upper_w > limits.max_contact_power_upper_w {
                reasons.push(RejectReason::ContactPowerExceeded);
            }
        }

        if !bundle.actuator.permit_active {
            reasons.push(RejectReason::ActuatorAuthorityAbsent);
        }
        if bundle.actuator.authority_epoch != context.current_actuator_authority_epoch {
            reasons.push(RejectReason::ActuatorAuthorityEpochMismatch);
        }
        reasons
    }
}

fn evidence_digests(bundle: EvidenceBundle) -> [Digest; 4] {
    [
        bundle.energy.measurement_digest,
        bundle.contact.measurement_digest,
        bundle.agency.authorization_digest,
        bundle.actuator.authority_digest,
    ]
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE_DIGEST: Digest = [0x91; 32];
    const STATE_DIGEST: Digest = [0x92; 32];
    const COMMAND_DIGEST: Digest = [0x93; 32];
    const PORT_GRAPH_DIGEST: Digest = [0x94; 32];
    const CALIBRATION_DIGEST: Digest = [0x95; 32];
    const SENSOR_PROFILE_DIGEST: Digest = [0x96; 32];
    const ENERGY_DIGEST: Digest = [0xA1; 32];
    const CONTACT_DIGEST: Digest = [0xA2; 32];
    const AUTHORIZATION_DIGEST: Digest = [0xA3; 32];
    const ACTUATOR_DIGEST: Digest = [0xA4; 32];
    const REARM_DIGEST: Digest = [0xA5; 32];

    fn profile() -> InteractionEnvelopeProfile {
        InteractionEnvelopeProfile {
            digest: PROFILE_DIGEST,
            limits: InteractionEnvelopeLimits {
                max_sample_age_ns: 500,
                max_sample_gap_ns: 200,
                max_energy_residual_upper_j: 0.1,
                max_energy_prefix_deficit_upper_j: 0.05,
                max_local_pressure_upper_pa: 10_000.0,
                max_pressure_time_dose_upper_pa_s: 20_000.0,
                max_deformation_rate_upper_s_inv: 2.0,
                max_contact_power_upper_w: 40.0,
            },
        }
    }

    fn identity(sequence: u64) -> EvidenceIdentity {
        EvidenceIdentity {
            sequence,
            proposal_sequence: sequence,
            sampled_at_ns: 1_000 + sequence.saturating_sub(1) * 10,
            proposal_expires_at_ns: 2_000 + sequence.saturating_sub(1) * 10,
            clock: ClockIdentity {
                clock_id: 7,
                boot_epoch: 9,
            },
            interaction_epoch: 1,
            state_revision_digest: STATE_DIGEST,
            profile_digest: PROFILE_DIGEST,
            command_digest: COMMAND_DIGEST,
            port_graph_digest: PORT_GRAPH_DIGEST,
        }
    }

    fn bundle(sequence: u64) -> EvidenceBundle {
        let identity = identity(sequence);
        EvidenceBundle {
            energy: EnergyEvidence {
                identity,
                residual_upper_j: 0.02,
                maximum_prefix_deficit_upper_j: 0.01,
                boundary_complete: true,
                measurement_digest: ENERGY_DIGEST,
            },
            contact: ContactEvidence {
                identity,
                local_pressure_upper_pa: 5_000.0,
                pressure_time_dose_upper_pa_s: 500.0,
                deformation_rate_upper_s_inv: 0.1,
                contact_power_upper_w: 10.0,
                calibration_digest: CALIBRATION_DIGEST,
                sensor_profile_digest: SENSOR_PROFILE_DIGEST,
                measurement_digest: CONTACT_DIGEST,
            },
            agency: AgencyEvidence {
                identity,
                agency_epoch: 1,
                explicit_user_authorization_active: true,
                stop_latched: false,
                authorization_digest: AUTHORIZATION_DIGEST,
            },
            actuator: ActuatorAuthorityEvidence {
                identity,
                authority_epoch: 4,
                permit_active: true,
                authority_digest: ACTUATOR_DIGEST,
            },
        }
    }

    fn context(sequence: u64) -> AdmissionContext {
        AdmissionContext {
            now_ns: 1_100 + sequence.saturating_sub(1) * 10,
            clock: ClockIdentity {
                clock_id: 7,
                boot_epoch: 9,
            },
            latest_issued_proposal_sequence: sequence,
            current_state_revision_digest: STATE_DIGEST,
            current_command_digest: COMMAND_DIGEST,
            current_port_graph_digest: PORT_GRAPH_DIGEST,
            current_actuator_authority_epoch: 4,
        }
    }

    fn gate() -> InteractionEnvelopeGate {
        InteractionEnvelopeGate::new(profile(), 1).unwrap()
    }

    fn rejects(
        bundle: EvidenceBundle,
        context: AdmissionContext,
        expected: RejectReason,
    ) {
        let mut gate = gate();
        let reasons = gate.admit(bundle, context).unwrap_err();
        assert!(
            reasons.contains(&expected),
            "missing {expected:?} in {reasons:?}"
        );
        assert_eq!(gate.last_admitted_sequence(), 0);
    }

    #[test]
    fn current_complete_bundle_is_admitted_and_revalidated_once() {
        let mut gate = gate();
        let evidence = bundle(1);
        let receipt = gate.admit(evidence, context(1)).unwrap();
        let valid = gate
            .revalidate_for_application(receipt, evidence, context(1))
            .unwrap();
        assert_eq!(valid.sequence(), 1);
        assert_eq!(valid.command_digest(), &COMMAND_DIGEST);
        assert_eq!(gate.last_admitted_sequence(), 1);
    }

    #[test]
    fn rejection_in_one_domain_cannot_be_compensated_by_another() {
        let mut evidence = bundle(1);
        evidence.energy.residual_upper_j = 1.0;
        evidence.contact.local_pressure_upper_pa = 1.0;
        rejects(evidence, context(1), RejectReason::EnergyResidualExceeded);
    }

    #[test]
    fn energy_boundary_must_be_complete() {
        let mut evidence = bundle(1);
        evidence.energy.boundary_complete = false;
        rejects(evidence, context(1), RejectReason::EnergyBoundaryIncomplete);
    }

    #[test]
    fn energy_residual_upper_bound_is_enforced() {
        let mut evidence = bundle(1);
        evidence.energy.residual_upper_j = 0.11;
        rejects(evidence, context(1), RejectReason::EnergyResidualExceeded);
    }

    #[test]
    fn historical_prefix_deficit_is_not_hidden_by_later_surplus() {
        let mut evidence = bundle(1);
        evidence.energy.residual_upper_j = -10.0;
        evidence.energy.maximum_prefix_deficit_upper_j = 0.06;
        rejects(evidence, context(1), RejectReason::EnergyPrefixDeficitExceeded);
    }

    #[test]
    fn invalid_energy_values_fail_closed() {
        let mut evidence = bundle(1);
        evidence.energy.maximum_prefix_deficit_upper_j = f64::NAN;
        rejects(evidence, context(1), RejectReason::InvalidEnergyEvidence);
    }

    #[test]
    fn each_contact_exposure_bound_is_independently_enforced() {
        let cases = [
            (0, RejectReason::LocalPressureExceeded),
            (1, RejectReason::PressureTimeDoseExceeded),
            (2, RejectReason::DeformationRateExceeded),
            (3, RejectReason::ContactPowerExceeded),
        ];
        for (field, expected) in cases {
            let mut evidence = bundle(1);
            match field {
                0 => evidence.contact.local_pressure_upper_pa = 10_000.1,
                1 => evidence.contact.pressure_time_dose_upper_pa_s = 20_000.1,
                2 => evidence.contact.deformation_rate_upper_s_inv = 2.1,
                _ => evidence.contact.contact_power_upper_w = 40.1,
            }
            rejects(evidence, context(1), expected);
        }
    }

    #[test]
    fn invalid_contact_measurements_fail_closed() {
        let mut evidence = bundle(1);
        evidence.contact.contact_power_upper_w = f64::INFINITY;
        rejects(evidence, context(1), RejectReason::InvalidContactEvidence);
    }

    #[test]
    fn tactile_authority_is_not_substituted_for_explicit_user_authorization() {
        let mut evidence = bundle(1);
        evidence.agency.explicit_user_authorization_active = false;
        rejects(
            evidence,
            context(1),
            RejectReason::ExplicitUserAuthorizationAbsent,
        );
    }

    #[test]
    fn stop_latch_dominates_every_other_valid_predicate() {
        let mut evidence = bundle(1);
        evidence.agency.stop_latched = true;
        rejects(evidence, context(1), RejectReason::StopLatched);
    }

    #[test]
    fn independent_actuator_authority_is_required() {
        let mut evidence = bundle(1);
        evidence.actuator.permit_active = false;
        rejects(evidence, context(1), RejectReason::ActuatorAuthorityAbsent);
    }

    #[test]
    fn actuator_authority_epoch_must_be_current() {
        let mut evidence = bundle(1);
        evidence.actuator.authority_epoch = 3;
        rejects(
            evidence,
            context(1),
            RejectReason::ActuatorAuthorityEpochMismatch,
        );
    }

    #[test]
    fn all_evidence_must_describe_the_same_command_and_state() {
        let mut evidence = bundle(1);
        evidence.contact.identity.command_digest[0] ^= 1;
        rejects(evidence, context(1), RejectReason::EvidenceIdentityMismatch);
    }

    #[test]
    fn wrong_clock_or_boot_epoch_is_rejected() {
        let mut evidence = bundle(1);
        evidence.energy.identity.clock.boot_epoch += 1;
        rejects(evidence, context(1), RejectReason::ClockMismatch);
    }

    #[test]
    fn wrong_profile_is_rejected() {
        let mut evidence = bundle(1);
        evidence.energy.identity.profile_digest[0] ^= 1;
        rejects(evidence, context(1), RejectReason::ProfileMismatch);
    }

    #[test]
    fn changed_current_state_revision_is_rejected() {
        let mut current = context(1);
        current.current_state_revision_digest[0] ^= 1;
        rejects(bundle(1), current, RejectReason::StateRevisionMismatch);
    }

    #[test]
    fn current_command_digest_is_checked_independently() {
        let mut current = context(1);
        current.current_command_digest[0] ^= 1;
        rejects(bundle(1), current, RejectReason::CommandMismatch);
    }

    #[test]
    fn zero_interaction_sequence_is_rejected() {
        let mut evidence = bundle(1);
        evidence.energy.identity.sequence = 0;
        rejects(evidence, context(1), RejectReason::ZeroInteractionSequence);
    }

    #[test]
    fn current_port_graph_digest_is_checked_against_authoritative_context() {
        let mut evidence = bundle(1);
        evidence.energy.identity.port_graph_digest[0] ^= 1;
        rejects(evidence, context(1), RejectReason::PortGraphMismatch);
    }

    #[test]
    fn delayed_optimizer_proposal_is_rejected() {
        let mut current = context(1);
        current.latest_issued_proposal_sequence = 2;
        rejects(bundle(1), current, RejectReason::ProposalNotLatestIssued);
    }

    #[test]
    fn expired_proposal_is_rejected() {
        let mut current = context(1);
        current.now_ns = 2_000;
        rejects(bundle(1), current, RejectReason::ProposalExpired);
    }

    #[test]
    fn stale_and_future_observations_are_rejected() {
        let mut current = context(1);
        current.now_ns = 1_501;
        rejects(bundle(1), current, RejectReason::ObservationStale);

        let mut future = bundle(1);
        future.energy.identity.sampled_at_ns = 1_201;
        let mut future_context = context(1);
        future_context.now_ns = 1_100;
        rejects(
            future,
            future_context,
            RejectReason::ObservationFromFuture,
        );
    }

    #[test]
    fn missing_provenance_digests_are_rejected() {
        let mut evidence = bundle(1);
        evidence.contact.calibration_digest = [0; 32];
        rejects(
            evidence,
            context(1),
            RejectReason::MissingCalibrationDigest,
        );
    }

    #[test]
    fn replayed_interaction_sequence_is_rejected() {
        let mut gate = gate();
        gate.admit(bundle(1), context(1)).unwrap();
        let reasons = gate.admit(bundle(1), context(1)).unwrap_err();
        assert!(reasons.contains(&RejectReason::InteractionSequenceReplay));
        assert_eq!(gate.last_admitted_sequence(), 1);
    }

    #[test]
    fn sample_gap_is_checked_against_previous_admission() {
        let mut gate = gate();
        gate.admit(bundle(1), context(1)).unwrap();
        let mut second = bundle(2);
        second.energy.identity.sampled_at_ns = 1_300;
        second.contact.identity.sampled_at_ns = 1_300;
        second.agency.identity.sampled_at_ns = 1_300;
        second.actuator.identity.sampled_at_ns = 1_300;
        let mut current = context(2);
        current.now_ns = 1_310;
        let reasons = gate.admit(second, current).unwrap_err();
        assert!(reasons.contains(&RejectReason::SampleGapExceeded));
        assert_eq!(gate.last_admitted_sequence(), 1);
    }

    #[test]
    fn rejected_evidence_does_not_advance_sequence_or_time() {
        let mut gate = gate();
        let mut invalid = bundle(1);
        invalid.contact.local_pressure_upper_pa = 50_000.0;
        assert!(gate.admit(invalid, context(1)).is_err());
        assert_eq!(gate.last_admitted_sequence(), 0);
        assert!(gate.admit(bundle(1), context(1)).is_ok());
    }

    #[test]
    fn a_new_admission_invalidates_an_older_receipt() {
        let mut gate = gate();
        let old = gate.admit(bundle(1), context(1)).unwrap();
        gate.admit(bundle(2), context(2)).unwrap();
        let reasons = gate
            .revalidate_for_application(old, bundle(1), context(1))
            .unwrap_err();
        assert!(reasons.contains(&RejectReason::ReceiptNotLatest));
    }

    #[test]
    fn evidence_digest_change_invalidates_the_receipt() {
        let mut gate = gate();
        let evidence = bundle(1);
        let receipt = gate.admit(evidence, context(1)).unwrap();
        let mut changed = evidence;
        changed.contact.measurement_digest[0] ^= 1;
        let reasons = gate
            .revalidate_for_application(receipt, changed, context(1))
            .unwrap_err();
        assert!(reasons.contains(&RejectReason::ReceiptEvidenceMismatch));
    }

    #[test]
    fn revalidation_rechecks_freshness_and_current_state() {
        let mut gate = gate();
        let evidence = bundle(1);
        let receipt = gate.admit(evidence, context(1)).unwrap();
        let mut current = context(1);
        current.now_ns = 1_501;
        let reasons = gate
            .revalidate_for_application(receipt, evidence, current)
            .unwrap_err();
        assert!(reasons.contains(&RejectReason::ObservationStale));
    }

    #[test]
    fn revocation_invalidates_old_receipts_and_requires_explicit_rearm() {
        let mut gate = gate();
        let evidence = bundle(1);
        let receipt = gate.admit(evidence, context(1)).unwrap();
        assert_eq!(gate.revoke_all().unwrap(), 2);
        assert!(gate.stop_latched());
        let reasons = gate
            .revalidate_for_application(receipt, evidence, context(1))
            .unwrap_err();
        assert!(reasons.contains(&RejectReason::StopLatched));
        assert!(gate
            .rearm_after_explicit_user_action(2, REARM_DIGEST)
            .is_ok());
        assert!(!gate.stop_latched());
    }

    #[test]
    fn rearm_rejects_missing_or_wrong_epoch_authorization() {
        let mut gate = gate();
        gate.revoke_all().unwrap();
        assert_eq!(
            gate.rearm_after_explicit_user_action(1, REARM_DIGEST),
            Err(RejectReason::RearmEpochMismatch)
        );
        assert_eq!(
            gate.rearm_after_explicit_user_action(2, [0; 32]),
            Err(RejectReason::RearmDigestMissing)
        );
        assert!(gate.stop_latched());
    }

    #[test]
    fn revalidation_consumes_receipt_once() {
        let mut gate = gate();
        let evidence = bundle(1);
        let receipt = gate.admit(evidence, context(1)).unwrap();
        let permit = gate
            .revalidate_for_application(receipt, evidence, context(1))
            .unwrap();
        assert_eq!(permit.receipt_id(), 1);
        assert_eq!(permit.sequence(), 1);
    }

    #[test]
    fn fixed_seed_generated_fault_combinations_have_zero_false_admissions() {
        fn next_u64(state: &mut u64) -> u64 {
            *state ^= *state << 13;
            *state ^= *state >> 7;
            *state ^= *state << 17;
            *state
        }

        const FAULTS: usize = 26;
        const CASES: usize = 30_000;
        let mut state = 20_261_112_u64;
        let mut coverage = [0_u32; FAULTS];
        let mut false_admissions = 0_u32;

        for _case in 0..CASES {
            let mut mask = (next_u64(&mut state) & ((1_u64 << FAULTS) - 1)) as u32;
            if mask == 0 {
                mask = 1_u32 << (next_u64(&mut state) as usize % FAULTS);
            }
            let mut evidence = bundle(1);
            let current = context(1);

            for (bit, seen) in coverage.iter_mut().enumerate() {
                if mask & (1_u32 << bit) == 0 {
                    continue;
                }
                *seen += 1;
                match bit {
                    0 => evidence.energy.residual_upper_j = 1.0,
                    1 => evidence.energy.maximum_prefix_deficit_upper_j = 1.0,
                    2 => evidence.energy.boundary_complete = false,
                    3 => evidence.contact.local_pressure_upper_pa = 20_000.0,
                    4 => evidence.contact.pressure_time_dose_upper_pa_s = 40_000.0,
                    5 => evidence.contact.deformation_rate_upper_s_inv = 4.0,
                    6 => evidence.contact.contact_power_upper_w = 80.0,
                    7 => evidence.contact.identity.command_digest[0] ^= 1,
                    8 => evidence.agency.explicit_user_authorization_active = false,
                    9 => evidence.agency.stop_latched = true,
                    10 => evidence.actuator.permit_active = false,
                    11 => evidence.actuator.authority_epoch += 1,
                    12 => evidence.energy.identity.clock.clock_id += 1,
                    13 => evidence.energy.identity.clock.boot_epoch += 1,
                    14 => evidence.energy.identity.profile_digest[0] ^= 1,
                    15 => evidence.energy.identity.state_revision_digest[0] ^= 1,
                    16 => evidence.energy.identity.port_graph_digest[0] ^= 1,
                    17 => evidence.energy.identity.proposal_sequence += 1,
                    18 => evidence.energy.identity.proposal_expires_at_ns = current.now_ns,
                    19 => {
                        if next_u64(&mut state) & 1 == 0 {
                            evidence.energy.identity.sampled_at_ns = current.now_ns + 1;
                        } else {
                            evidence.energy.identity.sampled_at_ns = current.now_ns - 501;
                        }
                    }
                    20 => evidence.energy.measurement_digest = [0; 32],
                    21 => evidence.contact.calibration_digest = [0; 32],
                    22 => evidence.contact.sensor_profile_digest = [0; 32],
                    23 => evidence.agency.authorization_digest = [0; 32],
                    24 => evidence.actuator.authority_digest = [0; 32],
                    25 => evidence.energy.identity.sequence = 0,
                    _ => unreachable!(),
                }
            }

            if gate().admit(evidence, current).is_ok() {
                false_admissions += 1;
            }
        }

        assert_eq!(coverage.iter().filter(|count| **count > 0).count(), FAULTS);
        assert_eq!(false_admissions, 0, "one or more generated invalid bundles were admitted");
    }

    #[test]
    fn invalid_profile_is_rejected_at_construction() {
        let mut invalid = profile();
        invalid.limits.max_local_pressure_upper_pa = f64::NAN;
        assert!(matches!(
            InteractionEnvelopeGate::new(invalid, 1),
            Err(RejectReason::InvalidProfile)
        ));
    }
}
