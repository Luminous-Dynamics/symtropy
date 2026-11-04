// SPDX-License-Identifier: AGPL-3.0-or-later
//! Research-only transition oracle for changing construction worksite context.
//! This crate has no hardware I/O and is not a robot safety controller.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    AwaitingEpoch,
    RequalificationRequired,
    Active,
    SafeHold(&'static str),
    ContextStale,
    ContextConflict,
    Aborted(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Epoch {
    pub revision: u64,
    pub digest: String,
    pub predecessor_digest: Option<String>,
    pub task_scope: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Qualification {
    pub epoch_digest: String,
    pub configuration_digest: String,
    pub task_scope: String,
    pub evidence_digest: String,
    pub record_reference: String,
    /// Generation invalidated by the most recent context/configuration/fault event.
    pub revalidation_generation: u64,
    /// Upstream evidence references created after that invalidation event.
    pub context_observation_reference: String,
    pub clock_assessment_reference: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreshnessPolicy {
    pub max_context_age_ns: u64,
    pub max_clock_uncertainty_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandBinding {
    pub epoch_digest: String,
    pub configuration_digest: String,
    pub task_scope: String,
    /// Measured age and uncertainty; compared to Model's pinned policy.
    pub context_age_ns: u64,
    pub clock_uncertainty_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuardError {
    InvalidDigest,
    InvalidTaskScope,
    UnauthenticatedEpoch,
    EpochRollbackOrConflict,
    EpochPredecessorMismatch,
    WrongTaskScope,
    NoCurrentEpoch,
    RequalificationNotPermitted,
    CommunicationsUnavailable,
    QualificationDoesNotMatchCurrentContext,
    QualificationEvidenceNotAccepted,
    StaleRevalidationProof,
    CommandNotAuthorized(&'static str),
    TerminalState,
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub struct Model {
    task_scope: String,
    configuration_digest: String,
    current_epoch: Option<Epoch>,
    qualification: Option<Qualification>,
    state: State,
    communications_available: bool,
    freshness_policy: FreshnessPolicy,
    invalidation_generation: u64,
}

impl Model {
    pub fn new(
        task_scope: impl Into<String>,
        configuration_digest: String,
        freshness_policy: FreshnessPolicy,
    ) -> Result<Self, GuardError> {
        let task_scope = task_scope.into();
        if task_scope.trim().is_empty() {
            return Err(GuardError::InvalidTaskScope);
        }
        if !valid_digest(&configuration_digest) {
            return Err(GuardError::InvalidDigest);
        }
        Ok(Self {
            task_scope,
            configuration_digest,
            current_epoch: None,
            qualification: None,
            state: State::AwaitingEpoch,
            communications_available: true,
            freshness_policy,
            invalidation_generation: 0,
        })
    }

    fn invalidate_qualification(&mut self) {
        self.qualification = None;
        self.invalidation_generation = self.invalidation_generation.saturating_add(1);
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn current_epoch(&self) -> Option<&Epoch> {
        self.current_epoch.as_ref()
    }

    fn conflict(&mut self) -> GuardError {
        self.invalidate_qualification();
        self.state = State::ContextConflict;
        GuardError::EpochRollbackOrConflict
    }

    /// Accept an epoch only after its signature/issuer/context is authenticated
    /// by the owning evidence service. The bool is an upstream assertion, not
    /// cryptographic verification performed by this reference model.
    pub fn accept_epoch(
        &mut self,
        epoch: Epoch,
        authenticated_by_upstream: bool,
    ) -> Result<(), GuardError> {
        if matches!(self.state, State::Aborted(_) | State::ContextConflict) {
            return Err(GuardError::TerminalState);
        }
        if !authenticated_by_upstream {
            self.invalidate_qualification();
            self.state = State::SafeHold("epoch_update_not_authenticated");
            return Err(GuardError::UnauthenticatedEpoch);
        }
        if !valid_digest(&epoch.digest)
            || epoch
                .predecessor_digest
                .as_deref()
                .is_some_and(|digest| !valid_digest(digest))
        {
            self.invalidate_qualification();
            self.state = State::SafeHold("epoch_digest_invalid");
            return Err(GuardError::InvalidDigest);
        }
        if epoch.task_scope != self.task_scope {
            return Err(self.conflict_or_wrong_scope());
        }
        match self.current_epoch.as_ref() {
            None => {
                if epoch.revision != 1 || epoch.predecessor_digest.is_some() {
                    return Err(self.conflict());
                }
                self.current_epoch = Some(epoch);
                self.invalidate_qualification();
                self.state = State::RequalificationRequired;
                Ok(())
            }
            Some(current)
                if epoch.revision == current.revision && epoch.digest == current.digest =>
            {
                if epoch.predecessor_digest == current.predecessor_digest {
                    Ok(()) // exact duplicate is idempotent
                } else {
                    Err(self.conflict())
                }
            }
            Some(current) => {
                if epoch.revision <= current.revision {
                    return Err(self.conflict());
                }
                if epoch.revision != current.revision + 1 {
                    return Err(self.conflict());
                }
                if epoch.predecessor_digest.as_deref() != Some(current.digest.as_str()) {
                    return Err(self.conflict());
                }
                self.current_epoch = Some(epoch);
                self.invalidate_qualification();
                self.state = State::RequalificationRequired;
                Ok(())
            }
        }
    }

    fn conflict_or_wrong_scope(&mut self) -> GuardError {
        self.invalidate_qualification();
        self.state = State::ContextConflict;
        GuardError::WrongTaskScope
    }

    pub fn mark_context_stale(&mut self) {
        if !matches!(self.state, State::Aborted(_) | State::ContextConflict) {
            self.invalidate_qualification();
            self.state = State::ContextStale;
        }
    }

    pub fn communications_lost(&mut self) {
        if self.communications_available {
            self.invalidate_qualification();
        }
        self.communications_available = false;
        if !matches!(self.state, State::Aborted(_) | State::ContextConflict) {
            self.state = State::SafeHold("communications_lost");
        }
    }

    /// Restoring transport alone never changes SAFE_HOLD to ACTIVE.
    pub fn communications_restored(&mut self) {
        self.communications_available = true;
    }

    /// Acknowledgement is deliberately non-authorizing.
    pub fn operator_acknowledged(&mut self) {}

    pub fn configuration_changed(&mut self, new_digest: String) -> Result<(), GuardError> {
        if matches!(self.state, State::Aborted(_) | State::ContextConflict) {
            return Err(GuardError::TerminalState);
        }
        if !valid_digest(&new_digest) {
            self.qualification = None;
            self.state = State::SafeHold("configuration_digest_invalid");
            return Err(GuardError::InvalidDigest);
        }
        if self.configuration_digest != new_digest {
            self.configuration_digest = new_digest;
            self.invalidate_qualification();
            self.state = State::RequalificationRequired;
        }
        Ok(())
    }

    /// Evidence acceptance is supplied by the upstream claim/evidence service.
    /// This method checks exact binding but does not verify signatures or actors.
    pub fn requalify(
        &mut self,
        proof: Qualification,
        accepted_by_upstream: bool,
    ) -> Result<(), GuardError> {
        if matches!(self.state, State::Aborted(_) | State::ContextConflict) {
            return Err(GuardError::RequalificationNotPermitted);
        }
        if !self.communications_available {
            return Err(GuardError::CommunicationsUnavailable);
        }
        let current = self
            .current_epoch
            .as_ref()
            .ok_or(GuardError::NoCurrentEpoch)?;
        if !matches!(
            self.state,
            State::RequalificationRequired | State::SafeHold(_) | State::ContextStale
        ) {
            return Err(GuardError::RequalificationNotPermitted);
        }
        if !accepted_by_upstream
            || !valid_digest(&proof.evidence_digest)
            || proof.record_reference.trim().is_empty()
            || proof.context_observation_reference.trim().is_empty()
            || proof.clock_assessment_reference.trim().is_empty()
        {
            return Err(GuardError::QualificationEvidenceNotAccepted);
        }
        if proof.revalidation_generation != self.invalidation_generation {
            return Err(GuardError::StaleRevalidationProof);
        }
        if proof.epoch_digest != current.digest
            || proof.configuration_digest != self.configuration_digest
            || proof.task_scope != self.task_scope
        {
            return Err(GuardError::QualificationDoesNotMatchCurrentContext);
        }
        self.qualification = Some(proof);
        self.state = State::Active;
        Ok(())
    }

    pub fn authorize(&self, command: &CommandBinding) -> Result<(), GuardError> {
        if self.state != State::Active {
            return Err(GuardError::CommandNotAuthorized("state_not_active"));
        }
        if !self.communications_available {
            return Err(GuardError::CommandNotAuthorized(
                "communications_unavailable",
            ));
        }
        if command.context_age_ns > self.freshness_policy.max_context_age_ns {
            return Err(GuardError::CommandNotAuthorized("context_age_exceeds_policy"));
        }
        if command.clock_uncertainty_ns > self.freshness_policy.max_clock_uncertainty_ns {
            return Err(GuardError::CommandNotAuthorized("clock_uncertainty_exceeds_policy"));
        }
        let current = self
            .current_epoch
            .as_ref()
            .ok_or(GuardError::NoCurrentEpoch)?;
        if command.epoch_digest != current.digest
            || command.configuration_digest != self.configuration_digest
            || command.task_scope != self.task_scope
        {
            return Err(GuardError::CommandNotAuthorized("command_binding_mismatch"));
        }
        let qualification = self
            .qualification
            .as_ref()
            .ok_or(GuardError::CommandNotAuthorized("qualification_missing"))?;
        if qualification.epoch_digest != current.digest
            || qualification.configuration_digest != self.configuration_digest
            || qualification.task_scope != self.task_scope
            || qualification.revalidation_generation != self.invalidation_generation
        {
            return Err(GuardError::CommandNotAuthorized(
                "qualification_binding_mismatch",
            ));
        }
        Ok(())
    }

    pub fn abort(&mut self, reason: &'static str) {
        self.invalidate_qualification();
        self.state = State::Aborted(reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn epoch(revision: u64, digest: &str, predecessor: Option<&str>) -> Epoch {
        Epoch {
            revision,
            digest: digest.to_owned(),
            predecessor_digest: predecessor.map(str::to_owned),
            task_scope: "floor-3-layout".to_owned(),
        }
    }

    fn model() -> Model {
        Model::new(
            "floor-3-layout",
            A.to_owned(),
            FreshnessPolicy {
                max_context_age_ns: 100,
                max_clock_uncertainty_ns: 10,
            },
        )
        .expect("valid model")
    }

    fn proof(model: &Model, epoch_digest: &str, config: &str) -> Qualification {
        Qualification {
            epoch_digest: epoch_digest.to_owned(),
            configuration_digest: config.to_owned(),
            task_scope: "floor-3-layout".to_owned(),
            evidence_digest: C.to_owned(),
            record_reference: "fixture-requalification-record".to_owned(),
            revalidation_generation: model.invalidation_generation,
            context_observation_reference: "fixture-fresh-context-observation".to_owned(),
            clock_assessment_reference: "fixture-clock-assessment".to_owned(),
        }
    }

    fn command(epoch_digest: &str, config: &str) -> CommandBinding {
        CommandBinding {
            epoch_digest: epoch_digest.to_owned(),
            configuration_digest: config.to_owned(),
            task_scope: "floor-3-layout".to_owned(),
            context_age_ns: 50,
            clock_uncertainty_ns: 5,
        }
    }

    fn qualify(
        model: &mut Model,
        epoch_digest: &str,
        config: &str,
    ) -> Result<(), GuardError> {
        let proof = proof(model, epoch_digest, config);
        model.requalify(proof, true)
    }

    #[test]
    fn genesis_epoch_requires_explicit_requalification() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true)
            .expect("accept genesis");
        assert_eq!(*m.state(), State::RequalificationRequired);
        assert!(m.authorize(&command(B, A)).is_err());
        qualify(&mut m, B, A).expect("requalify");
        assert_eq!(m.authorize(&command(B, A)), Ok(()));
    }

    #[test]
    fn new_epoch_invalidates_old_authorization_before_next_command() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        m.accept_epoch(epoch(2, C, Some(B)), true)
            .expect("revision 2");
        assert_eq!(*m.state(), State::RequalificationRequired);
        assert!(m.authorize(&command(B, A)).is_err());
        assert!(m.authorize(&command(C, A)).is_err());
        qualify(&mut m, C, A).expect("requalify latest");
        assert_eq!(m.authorize(&command(C, A)), Ok(()));
    }

    #[test]
    fn duplicate_epoch_is_idempotent_but_conflicting_digest_is_not() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        assert!(m.accept_epoch(epoch(1, B, None), true).is_ok());
        assert!(m.accept_epoch(epoch(1, C, None), true).is_err());
        assert_eq!(*m.state(), State::ContextConflict);
    }

    #[test]
    fn rollback_or_skipped_revision_fails_closed() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        assert!(m.accept_epoch(epoch(3, C, Some(B)), true).is_err());
        assert_eq!(*m.state(), State::ContextConflict);
    }

    #[test]
    fn comms_recovery_and_operator_ack_do_not_restore_authority() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        m.communications_lost();
        m.communications_restored();
        m.operator_acknowledged();
        assert_eq!(*m.state(), State::SafeHold("communications_lost"));
        assert!(m.authorize(&command(B, A)).is_err());
        qualify(&mut m, B, A).expect("explicit requalify");
        assert_eq!(*m.state(), State::Active);
    }

    #[test]
    fn configuration_change_requires_requalification() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        m.configuration_changed(C.to_owned())
            .expect("config change");
        assert_eq!(*m.state(), State::RequalificationRequired);
        assert!(m.authorize(&command(B, A)).is_err());
        assert!(qualify(&mut m, B, A).is_err());
        qualify(&mut m, B, C).expect("requalify config");
        assert_eq!(m.authorize(&command(B, C)), Ok(()));
    }

    #[test]
    fn unauthenticated_epoch_never_replaces_the_current_context() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        assert_eq!(
            m.accept_epoch(epoch(2, C, Some(B)), false),
            Err(GuardError::UnauthenticatedEpoch)
        );
        assert_eq!(m.current_epoch().expect("current epoch").digest, B);
        assert!(m.authorize(&command(B, A)).is_err());
    }

    #[test]
    fn stale_context_and_clock_uncertainty_block_commands() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        let mut stale = command(B, A);
        stale.context_age_ns = 101;
        assert!(m.authorize(&stale).is_err());
        let mut uncertain_clock = command(B, A);
        uncertain_clock.clock_uncertainty_ns = 11;
        assert!(m.authorize(&uncertain_clock).is_err());
        assert_eq!(m.authorize(&command(B, A)), Ok(()));
    }

    #[test]
    fn pre_invalidation_requalification_proof_cannot_restore_active() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        let old_proof = proof(&m, B, A);
        m.mark_context_stale();
        assert_eq!(
            m.requalify(old_proof, true),
            Err(GuardError::StaleRevalidationProof)
        );
        qualify(&mut m, B, A).expect("fresh revalidation");
        assert_eq!(*m.state(), State::Active);
    }

    #[test]
    fn unauthenticated_epoch_update_invalidates_old_requalification_proof() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        let old_proof = proof(&m, B, A);
        assert_eq!(
            m.accept_epoch(epoch(2, C, Some(B)), false),
            Err(GuardError::UnauthenticatedEpoch)
        );
        assert_eq!(
            m.requalify(old_proof, true),
            Err(GuardError::StaleRevalidationProof)
        );
        m.accept_epoch(epoch(2, C, Some(B)), true).expect("authenticated current epoch");
        qualify(&mut m, C, A).expect("fresh current-context proof");
        assert_eq!(*m.state(), State::Active);
    }

    #[test]
    fn requalification_requires_fresh_context_and_clock_records() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        let mut missing = proof(&m, B, A);
        missing.context_observation_reference.clear();
        assert_eq!(
            m.requalify(missing, true),
            Err(GuardError::QualificationEvidenceNotAccepted)
        );
        let mut missing_clock = proof(&m, B, A);
        missing_clock.clock_assessment_reference.clear();
        assert_eq!(
            m.requalify(missing_clock, true),
            Err(GuardError::QualificationEvidenceNotAccepted)
        );
    }

    #[test]
    fn abort_is_latched_and_cannot_be_cleared_by_requalification() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        m.abort("fixture_abort");
        assert_eq!(*m.state(), State::Aborted("fixture_abort"));
        assert_eq!(
            qualify(&mut m, B, A),
            Err(GuardError::RequalificationNotPermitted)
        );
    }
}
