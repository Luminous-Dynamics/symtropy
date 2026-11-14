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
    /// Monotonic clock domain/boot identity that issued this lease.
    pub timebase_id: String,
    pub issued_at_monotonic_ns: u64,
    pub valid_until_monotonic_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreshnessPolicy {
    pub max_context_age_ns: u64,
    pub max_clock_uncertainty_ns: u64,
    pub max_qualification_lease_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandBinding {
    pub epoch_digest: String,
    pub configuration_digest: String,
    pub task_scope: String,
    /// Measured age and uncertainty; compared to Model's pinned policy.
    pub context_age_ns: u64,
    pub clock_uncertainty_ns: u64,
    pub timebase_id: String,
    pub now_monotonic_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuardError {
    InvalidDigest,
    InvalidTaskScope,
    InvalidTimebaseId,
    UnauthenticatedEpoch,
    EpochRollbackOrConflict,
    EpochPredecessorMismatch,
    WrongTaskScope,
    NoCurrentEpoch,
    RequalificationNotPermitted,
    CommunicationsUnavailable,
    TimebaseMismatch,
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

#[derive(Clone)]
pub struct Model {
    task_scope: String,
    configuration_digest: String,
    current_epoch: Option<Epoch>,
    qualification: Option<Qualification>,
    state: State,
    communications_available: bool,
    freshness_policy: FreshnessPolicy,
    timebase_id: String,
    invalidation_generation: u64,
}

impl Model {
    pub fn new(
        task_scope: impl Into<String>,
        configuration_digest: String,
        timebase_id: String,
        freshness_policy: FreshnessPolicy,
    ) -> Result<Self, GuardError> {
        let task_scope = task_scope.into();
        if task_scope.trim().is_empty() {
            return Err(GuardError::InvalidTaskScope);
        }
        if !valid_digest(&configuration_digest) {
            return Err(GuardError::InvalidDigest);
        }
        if timebase_id.trim().is_empty() {
            return Err(GuardError::InvalidTimebaseId);
        }
        if freshness_policy.max_qualification_lease_ns == 0 {
            return Err(GuardError::QualificationEvidenceNotAccepted);
        }
        Ok(Self {
            task_scope,
            configuration_digest,
            current_epoch: None,
            qualification: None,
            state: State::AwaitingEpoch,
            communications_available: true,
            freshness_policy,
            timebase_id,
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
            self.invalidate_qualification();
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
        if proof.timebase_id != self.timebase_id {
            return Err(GuardError::TimebaseMismatch);
        }
        if proof.valid_until_monotonic_ns <= proof.issued_at_monotonic_ns
            || proof.valid_until_monotonic_ns - proof.issued_at_monotonic_ns
                > self.freshness_policy.max_qualification_lease_ns
        {
            return Err(GuardError::QualificationEvidenceNotAccepted);
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

    pub fn authorize(&mut self, command: &CommandBinding) -> Result<(), GuardError> {
        if self.state != State::Active {
            return Err(GuardError::CommandNotAuthorized("state_not_active"));
        }
        if !self.communications_available {
            return Err(GuardError::CommandNotAuthorized(
                "communications_unavailable",
            ));
        }
        if command.context_age_ns > self.freshness_policy.max_context_age_ns {
            self.mark_context_stale();
            return Err(GuardError::CommandNotAuthorized(
                "context_age_exceeds_policy",
            ));
        }
        if command.clock_uncertainty_ns > self.freshness_policy.max_clock_uncertainty_ns {
            self.mark_context_stale();
            return Err(GuardError::CommandNotAuthorized(
                "clock_uncertainty_exceeds_policy",
            ));
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
        let lease_check = {
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
            qualification.timebase_id != self.timebase_id
                || command.timebase_id != self.timebase_id
                || command.now_monotonic_ns < qualification.issued_at_monotonic_ns
                || command.now_monotonic_ns >= qualification.valid_until_monotonic_ns
        };
        if lease_check {
            self.mark_context_stale();
            return Err(GuardError::CommandNotAuthorized(
                "qualification_lease_expired_or_timebase_mismatch",
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
            "boot-001".to_owned(),
            FreshnessPolicy {
                max_context_age_ns: 100,
                max_clock_uncertainty_ns: 10,
                max_qualification_lease_ns: 1_000,
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
            timebase_id: model.timebase_id.clone(),
            issued_at_monotonic_ns: 1_000,
            valid_until_monotonic_ns: 2_000,
        }
    }

    fn command(epoch_digest: &str, config: &str) -> CommandBinding {
        CommandBinding {
            epoch_digest: epoch_digest.to_owned(),
            configuration_digest: config.to_owned(),
            task_scope: "floor-3-layout".to_owned(),
            context_age_ns: 50,
            clock_uncertainty_ns: 5,
            timebase_id: "boot-001".to_owned(),
            now_monotonic_ns: 1_500,
        }
    }

    fn qualify(model: &mut Model, epoch_digest: &str, config: &str) -> Result<(), GuardError> {
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
    fn invalid_configuration_digest_invalidates_old_requalification_proof() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        let old_proof = proof(&m, B, A);
        assert_eq!(
            m.configuration_changed("not-a-digest".to_owned()),
            Err(GuardError::InvalidDigest)
        );
        assert_eq!(*m.state(), State::SafeHold("configuration_digest_invalid"));
        assert_eq!(
            m.requalify(old_proof, true),
            Err(GuardError::StaleRevalidationProof)
        );
        qualify(&mut m, B, A).expect("fresh requalification after configuration fault");
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
        let mut stale_model = model();
        stale_model
            .accept_epoch(epoch(1, B, None), true)
            .expect("genesis");
        qualify(&mut stale_model, B, A).expect("qualify stale-context model");
        let mut stale = command(B, A);
        stale.context_age_ns = 101;
        assert!(stale_model.authorize(&stale).is_err());
        assert_eq!(*stale_model.state(), State::ContextStale);
        assert!(stale_model.authorize(&command(B, A)).is_err());

        let mut clock_model = model();
        clock_model
            .accept_epoch(epoch(1, B, None), true)
            .expect("genesis");
        qualify(&mut clock_model, B, A).expect("qualify clock model");
        let mut uncertain_clock = command(B, A);
        uncertain_clock.clock_uncertainty_ns = 11;
        assert!(clock_model.authorize(&uncertain_clock).is_err());
        assert_eq!(*clock_model.state(), State::ContextStale);
        assert!(clock_model.authorize(&command(B, A)).is_err());
    }

    #[test]
    fn observed_context_expiry_latches_safe_stale_state() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        let mut expired = command(B, A);
        expired.context_age_ns = 101;
        assert!(m.authorize(&expired).is_err());
        assert_eq!(*m.state(), State::ContextStale);
        assert!(m.authorize(&command(B, A)).is_err());
    }

    #[test]
    fn observed_clock_uncertainty_latches_safe_stale_state() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        let mut uncertain = command(B, A);
        uncertain.clock_uncertainty_ns = 11;
        assert!(m.authorize(&uncertain).is_err());
        assert_eq!(*m.state(), State::ContextStale);
        assert!(m.authorize(&command(B, A)).is_err());
    }

    #[test]
    fn qualification_lease_expiry_latches_safe_stale_state() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        let mut expired = command(B, A);
        expired.now_monotonic_ns = 2_000;
        assert!(m.authorize(&expired).is_err());
        assert_eq!(*m.state(), State::ContextStale);
        assert!(m.authorize(&command(B, A)).is_err());
    }

    #[test]
    fn changed_timebase_cannot_reuse_a_previous_qualification() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        qualify(&mut m, B, A).expect("qualify");
        let mut changed_timebase = command(B, A);
        changed_timebase.timebase_id = "boot-002".to_owned();
        assert!(m.authorize(&changed_timebase).is_err());
        assert_eq!(*m.state(), State::ContextStale);
    }

    #[test]
    fn requalification_cannot_issue_a_lease_beyond_policy() {
        let mut m = model();
        m.accept_epoch(epoch(1, B, None), true).expect("genesis");
        let mut overlong = proof(&m, B, A);
        overlong.valid_until_monotonic_ns = 2_001;
        assert_eq!(
            m.requalify(overlong, true),
            Err(GuardError::QualificationEvidenceNotAccepted)
        );
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
        m.accept_epoch(epoch(2, C, Some(B)), true)
            .expect("authenticated current epoch");
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
    fn bounded_event_sequences_preserve_active_lease_invariants() {
        // Exhaust all action sequences of length four over eleven event classes.
        // This checks 11^4 = 14,641 bounded traces (plus every prefix).
        const DEPTH: usize = 4;
        const ACTIONS: usize = 11;

        fn assert_invariants(model: &mut Model) {
            if model.state == State::Active {
                let current = model
                    .current_epoch
                    .as_ref()
                    .expect("active requires an epoch");
                let proof = model
                    .qualification
                    .as_ref()
                    .expect("active requires qualification");
                assert!(
                    model.communications_available,
                    "active requires communications"
                );
                assert_eq!(proof.epoch_digest, current.digest);
                assert_eq!(proof.configuration_digest, model.configuration_digest);
                assert_eq!(proof.task_scope, model.task_scope);
                assert_eq!(proof.revalidation_generation, model.invalidation_generation);
                assert_eq!(proof.timebase_id, model.timebase_id);
                assert!(proof.valid_until_monotonic_ns > proof.issued_at_monotonic_ns);
                assert!(
                    proof.valid_until_monotonic_ns - proof.issued_at_monotonic_ns
                        <= model.freshness_policy.max_qualification_lease_ns
                );
                let valid = CommandBinding {
                    epoch_digest: current.digest.clone(),
                    configuration_digest: model.configuration_digest.clone(),
                    task_scope: model.task_scope.clone(),
                    context_age_ns: 0,
                    clock_uncertainty_ns: 0,
                    timebase_id: model.timebase_id.clone(),
                    now_monotonic_ns: proof.issued_at_monotonic_ns,
                };
                assert_eq!(model.authorize(&valid), Ok(()));
            } else if let Some(current) = model.current_epoch.as_ref() {
                let denied = CommandBinding {
                    epoch_digest: current.digest.clone(),
                    configuration_digest: model.configuration_digest.clone(),
                    task_scope: model.task_scope.clone(),
                    context_age_ns: 0,
                    clock_uncertainty_ns: 0,
                    timebase_id: model.timebase_id.clone(),
                    now_monotonic_ns: 1_500,
                };
                assert!(model.authorize(&denied).is_err());
            }
        }

        fn walk(model: Model, remaining: usize, visited: &mut usize) {
            *visited += 1;
            let mut checked = model.clone();
            assert_invariants(&mut checked);
            if remaining == 0 {
                return;
            }

            for action in 0..ACTIONS {
                let mut next = model.clone();
                match action {
                    // Accept the next correctly linked context epoch.
                    0 => {
                        let (revision, predecessor) = match next.current_epoch.as_ref() {
                            Some(current) => (
                                current.revision.checked_add(1).unwrap_or(current.revision),
                                Some(current.digest.clone()),
                            ),
                            None => (1, None),
                        };
                        let digest = format!("{:064x}", revision.saturating_add(0x1000));
                        let _ = next
                            .accept_epoch(epoch(revision, &digest, predecessor.as_deref()), true);
                    }
                    1 => next.mark_context_stale(),
                    2 => next.communications_lost(),
                    3 => next.communications_restored(),
                    4 => next.operator_acknowledged(),
                    5 => {
                        let replacement = if next.configuration_digest == A { C } else { A };
                        let _ = next.configuration_changed(replacement.to_owned());
                    }
                    6 => {
                        if let Some(current) = next.current_epoch.as_ref() {
                            let digest = current.digest.clone();
                            let config = next.configuration_digest.clone();
                            let _ = qualify(&mut next, &digest, &config);
                        }
                    }
                    7 => {
                        if let Some(current) = next.current_epoch.as_ref() {
                            let binding = CommandBinding {
                                epoch_digest: current.digest.clone(),
                                configuration_digest: next.configuration_digest.clone(),
                                task_scope: next.task_scope.clone(),
                                context_age_ns: 0,
                                clock_uncertainty_ns: 0,
                                timebase_id: next.timebase_id.clone(),
                                now_monotonic_ns: 1_500,
                            };
                            let _ = next.authorize(&binding);
                        }
                    }
                    8 => next.abort("bounded_sequence_abort"),
                    9 => {
                        let (revision, predecessor) = match next.current_epoch.as_ref() {
                            Some(current) => (
                                current.revision.saturating_add(1),
                                Some(current.digest.clone()),
                            ),
                            None => (1, None),
                        };
                        let digest = format!("{:064x}", revision.saturating_add(0x2000));
                        let _ = next
                            .accept_epoch(epoch(revision, &digest, predecessor.as_deref()), false);
                    }
                    10 => {
                        if let Some(current) = next.current_epoch.as_ref() {
                            let wrong_digest = if current.digest == B { C } else { B };
                            let binding = CommandBinding {
                                epoch_digest: wrong_digest.to_owned(),
                                configuration_digest: next.configuration_digest.clone(),
                                task_scope: next.task_scope.clone(),
                                context_age_ns: 0,
                                clock_uncertainty_ns: 0,
                                timebase_id: next.timebase_id.clone(),
                                now_monotonic_ns: 1_500,
                            };
                            let _ = next.authorize(&binding);
                        }
                    }
                    _ => unreachable!(),
                }
                walk(next, remaining - 1, visited);
            }
        }

        let mut visited = 0;
        walk(model(), DEPTH, &mut visited);
        assert!(visited > 14_641);
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

#[cfg(test)]
mod frozen_scenario_tests {
    use serde::Deserialize;

    use super::{CommandBinding, Epoch, FreshnessPolicy, Model, Qualification, State};

    const ORDERED: &str = include_str!(
        "../../../../research/construction-worksite-epoch-transition-benchmark-v0-1/scenarios/ordered-epoch-change.json"
    );
    const REPLAY: &str = include_str!(
        "../../../../research/construction-worksite-epoch-transition-benchmark-v0-1/scenarios/replayed-epoch-fails-closed.json"
    );
    const RECONNECT: &str = include_str!(
        "../../../../research/construction-worksite-epoch-transition-benchmark-v0-1/scenarios/reconnect-needs-requalification.json"
    );
    const EXPIRED: &str = include_str!(
        "../../../../research/construction-worksite-epoch-transition-benchmark-v0-1/scenarios/qualification-lease-expiry-fails-closed.json"
    );
    const SCENARIO_SCHEMA: &str = include_str!(
        "../../../../research/construction-worksite-epoch-transition-benchmark-v0-1/scenarios/worksite-epoch-scenario-v0.1.schema.json"
    );

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Scenario {
        schema_version: String,
        scenario_id: String,
        task_scope: String,
        initial_configuration_digest: String,
        timebase_id: String,
        max_context_age_ns: u64,
        max_clock_uncertainty_ns: u64,
        max_qualification_lease_ns: u64,
        expected_final_state: String,
        events: Vec<ScenarioEvent>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
    enum ScenarioEvent {
        AcceptEpoch {
            revision: u64,
            digest: String,
            predecessor_digest: Option<String>,
            task_scope: String,
            authenticated: bool,
            expected: String,
        },
        Requalify {
            epoch_digest: String,
            configuration_digest: String,
            task_scope: String,
            evidence_digest: String,
            record_reference: String,
            accepted_by_upstream: bool,
            revalidation_generation: u64,
            context_observation_reference: String,
            clock_assessment_reference: String,
            timebase_id: String,
            issued_at_monotonic_ns: u64,
            valid_until_monotonic_ns: u64,
            expected: String,
        },
        Command {
            epoch_digest: String,
            configuration_digest: String,
            task_scope: String,
            context_age_ns: u64,
            clock_uncertainty_ns: u64,
            timebase_id: String,
            now_monotonic_ns: u64,
            expected: String,
        },
        CommunicationsLost,
        CommunicationsRestored,
        OperatorAcknowledged,
        MarkContextStale,
        ConfigurationChanged {
            new_digest: String,
            expected: String,
        },
    }

    fn expected_bool(scenario: &str, index: usize, actual: bool, expected: &str) {
        let expected_result = match expected {
            "accepted" | "activated" | "authorized" => true,
            "rejected" | "denied" => false,
            other => panic!("scenario {scenario} event {index}: unknown expected result {other}"),
        };
        assert_eq!(
            actual, expected_result,
            "scenario {scenario} event {index} expected {expected}"
        );
    }

    fn state_name(state: &State) -> &'static str {
        match state {
            State::AwaitingEpoch => "awaiting_epoch",
            State::RequalificationRequired => "requalification_required",
            State::Active => "active",
            State::SafeHold(_) => "safe_hold",
            State::ContextStale => "context_stale",
            State::ContextConflict => "context_conflict",
            State::Aborted(_) => "aborted",
        }
    }

    fn replay_scenario(raw: &str) {
        let scenario: Scenario = serde_json::from_str(raw).expect("strict scenario JSON");
        assert_eq!(
            scenario.schema_version, "construction-worksite-epoch-scenario/0.1",
            "scenario {} schema version",
            scenario.scenario_id
        );
        let mut model = Model::new(
            scenario.task_scope.clone(),
            scenario.initial_configuration_digest,
            scenario.timebase_id.clone(),
            FreshnessPolicy {
                max_context_age_ns: scenario.max_context_age_ns,
                max_clock_uncertainty_ns: scenario.max_clock_uncertainty_ns,
                max_qualification_lease_ns: scenario.max_qualification_lease_ns,
            },
        )
        .expect("scenario model");

        for (index, event) in scenario.events.into_iter().enumerate() {
            match event {
                ScenarioEvent::AcceptEpoch {
                    revision,
                    digest,
                    predecessor_digest,
                    task_scope,
                    authenticated,
                    expected,
                } => {
                    let result = model.accept_epoch(
                        Epoch {
                            revision,
                            digest,
                            predecessor_digest,
                            task_scope,
                        },
                        authenticated,
                    );
                    expected_bool(&scenario.scenario_id, index, result.is_ok(), &expected);
                }
                ScenarioEvent::Requalify {
                    epoch_digest,
                    configuration_digest,
                    task_scope,
                    evidence_digest,
                    record_reference,
                    accepted_by_upstream,
                    revalidation_generation,
                    context_observation_reference,
                    clock_assessment_reference,
                    timebase_id,
                    issued_at_monotonic_ns,
                    valid_until_monotonic_ns,
                    expected,
                } => {
                    let result = model.requalify(
                        Qualification {
                            epoch_digest,
                            configuration_digest,
                            task_scope,
                            evidence_digest,
                            record_reference,
                            revalidation_generation,
                            context_observation_reference,
                            clock_assessment_reference,
                            timebase_id,
                            issued_at_monotonic_ns,
                            valid_until_monotonic_ns,
                        },
                        accepted_by_upstream,
                    );
                    expected_bool(&scenario.scenario_id, index, result.is_ok(), &expected);
                }
                ScenarioEvent::Command {
                    epoch_digest,
                    configuration_digest,
                    task_scope,
                    context_age_ns,
                    clock_uncertainty_ns,
                    timebase_id,
                    now_monotonic_ns,
                    expected,
                } => {
                    let result = model.authorize(&CommandBinding {
                        epoch_digest,
                        configuration_digest,
                        task_scope,
                        context_age_ns,
                        clock_uncertainty_ns,
                        timebase_id,
                        now_monotonic_ns,
                    });
                    expected_bool(&scenario.scenario_id, index, result.is_ok(), &expected);
                }
                ScenarioEvent::CommunicationsLost => model.communications_lost(),
                ScenarioEvent::CommunicationsRestored => model.communications_restored(),
                ScenarioEvent::OperatorAcknowledged => model.operator_acknowledged(),
                ScenarioEvent::MarkContextStale => model.mark_context_stale(),
                ScenarioEvent::ConfigurationChanged {
                    new_digest,
                    expected,
                } => {
                    let result = model.configuration_changed(new_digest);
                    expected_bool(&scenario.scenario_id, index, result.is_ok(), &expected);
                }
            }
        }

        assert_eq!(
            state_name(model.state()),
            scenario.expected_final_state,
            "scenario {} final state",
            scenario.scenario_id
        );
    }

    #[test]
    fn frozen_transition_scenarios_replay_to_expected_decisions() {
        for fixture in [ORDERED, REPLAY, RECONNECT, EXPIRED] {
            replay_scenario(fixture);
        }
    }

    #[test]
    fn scenario_schema_is_parseable_and_closed() {
        let schema: serde_json::Value =
            serde_json::from_str(SCENARIO_SCHEMA).expect("scenario schema JSON");
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(schema["additionalProperties"], false);
        assert!(schema["properties"]["events"]["items"]["oneOf"].is_array());
    }

    #[test]
    fn every_frozen_scenario_validates_against_draft_2020_12_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(SCENARIO_SCHEMA).expect("scenario schema JSON");
        jsonschema::draft202012::meta::validate(&schema)
            .expect("scenario schema must be valid Draft 2020-12");

        let validator = jsonschema::draft202012::new(&schema).expect("compile scenario schema");
        for (name, raw) in [
            ("ordered", ORDERED),
            ("replay", REPLAY),
            ("reconnect", RECONNECT),
            ("lease-expiry", EXPIRED),
        ] {
            let instance: serde_json::Value =
                serde_json::from_str(raw).expect("scenario fixture JSON");
            let errors = validator
                .iter_errors(&instance)
                .map(|error| format!("{error} at {}", error.instance_path()))
                .collect::<Vec<_>>();
            assert!(
                errors.is_empty(),
                "scenario {name} violates the frozen schema: {}",
                errors.join("; ")
            );
        }
    }

    #[test]
    fn frozen_scenario_parser_rejects_unknown_fields() {
        let malformed = ORDERED.replace(
            "\"revision\": 1,",
            "\"revision\": 1, \"silent_fallback\": true,",
        );
        assert!(
            serde_json::from_str::<Scenario>(&malformed).is_err(),
            "unknown scenario fields must not be silently ignored"
        );
    }
}
