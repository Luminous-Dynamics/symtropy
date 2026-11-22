---- MODULE RestartSafeEffectContinuity ----
EXTENDS Naturals, TLC

CONSTANTS MaxEpoch, NaiveRetryEnabled

ASSUME MaxEpoch \in Nat /\ MaxEpoch >= 3
ASSUME NaiveRetryEnabled \in BOOLEAN

OldAttemptId == "attempt-1"
RecoveryAttemptId == "attempt-2"

vars == <<s>>

Init ==
    s =
        [ processStatus |-> "running",
          bootEpoch |-> 1,
          controllerEpoch |-> 1,
          stage |-> "unreserved",
          reservationCommitted |-> FALSE,
          oldTicketBootEpoch |-> 1,
          oldTicketControllerEpoch |-> 1,
          oldAccepted |-> FALSE,
          oldAcceptedBootEpoch |-> 0,
          oldAcceptedControllerEpoch |-> 0,
          commandPending |-> FALSE,
          commandOwnerAttempt |-> "none",
          oldPhysicalOutcome |-> "none",
          recoveryPhysicalOutcome |-> "none",
          appliedEffectCount |-> 0,
          persistStatus |-> "definitely_not_committed",
          storeRecord |-> FALSE,
          storeCallPending |-> FALSE,
          effectEvidence |-> "none",
          missionResolution |-> "none",
          observerIndependent |-> TRUE,
          queueFenceBootEpoch |-> 0,
          downstreamFenceBootEpoch |-> 0,
          recoveryPermit |-> "absent",
          permitUseCount |-> 0,
          ticketStatus |-> "old",
          ticketBootEpoch |-> 1,
          ticketControllerEpoch |-> 1,
          ticketAttempt |-> OldAttemptId,
          m1Current |-> TRUE,
          m2Current |-> TRUE,
          ticketBindingValid |-> TRUE,
          liveBindingValid |-> TRUE,
          recoveryAccepted |-> FALSE,
          admittedFreshBoot |-> FALSE,
          admittedFreshController |-> FALSE,
          admittedControllerFence |-> FALSE,
          admittedDownstreamFence |-> FALSE,
          admittedM1 |-> FALSE,
          admittedM2 |-> FALSE,
          admittedBinding |-> FALSE,
          naiveReplayCount |-> 0,
          crashCut |-> "none" ]

ReserveAttempt ==
    /\ s.processStatus = "running"
    /\ s.stage = "unreserved"
    /\ s' = [s EXCEPT
                !.stage = "reserved",
                !.reservationCommitted = TRUE]

MarkDispatchStarted ==
    /\ s.processStatus = "running"
    /\ s.stage = "reserved"
    /\ s.reservationCommitted
    /\ s' = [s EXCEPT !.stage = "dispatch_started"]

NativeAcceptOld ==
    /\ s.processStatus = "running"
    /\ s.stage = "dispatch_started"
    /\ s.reservationCommitted
    /\ s.bootEpoch = 1
    /\ s.controllerEpoch = 1
    /\ s.oldTicketBootEpoch = s.bootEpoch
    /\ s.oldTicketControllerEpoch = s.controllerEpoch
    /\ ~s.oldAccepted
    /\ ~s.commandPending
    /\ s' = [s EXCEPT
                !.stage = "accepted",
                !.oldAccepted = TRUE,
                !.oldAcceptedBootEpoch = s.bootEpoch,
                !.oldAcceptedControllerEpoch = s.controllerEpoch,
                !.commandPending = TRUE,
                !.commandOwnerAttempt = OldAttemptId]

OldDeviceApplies ==
    /\ s.commandPending
    /\ s.commandOwnerAttempt = OldAttemptId
    /\ s.downstreamFenceBootEpoch # s.bootEpoch
    /\ s.oldPhysicalOutcome = "none"
    /\ s' = [s EXCEPT
                !.oldPhysicalOutcome = "applied",
                !.appliedEffectCount = @ + 1,
                !.commandPending = FALSE,
                !.commandOwnerAttempt = "none"]

OldDeviceDoesNotApply ==
    /\ s.commandPending
    /\ s.commandOwnerAttempt = OldAttemptId
    /\ s.downstreamFenceBootEpoch # s.bootEpoch
    /\ s.oldPhysicalOutcome = "none"
    /\ s' = [s EXCEPT
                !.oldPhysicalOutcome = "no_effect",
                !.commandPending = FALSE,
                !.commandOwnerAttempt = "none"]

OldDevicePartiallyApplies ==
    /\ s.commandPending
    /\ s.commandOwnerAttempt = OldAttemptId
    /\ s.downstreamFenceBootEpoch # s.bootEpoch
    /\ s.oldPhysicalOutcome = "none"
    /\ s' = [s EXCEPT
                !.oldPhysicalOutcome = "partial",
                !.commandPending = FALSE,
                !.commandOwnerAttempt = "none"]

BeginEffectRecordCommit ==
    /\ s.processStatus = "running"
    /\ s.oldPhysicalOutcome # "none"
    /\ s.persistStatus = "definitely_not_committed"
    /\ ~s.storeCallPending
    /\ ~s.storeRecord
    /\ s' = [s EXCEPT
                !.persistStatus = "in_flight",
                !.storeCallPending = TRUE]

CommitEffectRecord ==
    /\ s.storeCallPending
    /\ ~s.storeRecord
    /\ s' = [s EXCEPT
                !.storeRecord = TRUE,
                !.storeCallPending = FALSE]

AbortEffectRecord ==
    /\ s.storeCallPending
    /\ ~s.storeRecord
    /\ s' = [s EXCEPT !.storeCallPending = FALSE]

AcknowledgeStoreCommit ==
    /\ s.processStatus = "running"
    /\ s.persistStatus = "in_flight"
    /\ s.storeRecord
    /\ ~s.storeCallPending
    /\ s' = [s EXCEPT !.persistStatus = "committed"]

AcknowledgeStoreNoCommit ==
    /\ s.processStatus = "running"
    /\ s.persistStatus = "in_flight"
    /\ ~s.storeRecord
    /\ ~s.storeCallPending
    /\ s' = [s EXCEPT !.persistStatus = "definitely_not_committed"]

Crash ==
    /\ s.processStatus = "running"
    /\ s' = [s EXCEPT
                !.processStatus = "crashed",
                !.persistStatus =
                    IF @ = "in_flight"
                    THEN "indeterminate_commit"
                    ELSE @,
                !.crashCut =
                    IF s.recoveryAccepted /\ s.recoveryPhysicalOutcome = "none"
                    THEN "permit_consumed_before_effect"
                    ELSE IF s.oldPhysicalOutcome # "none"
                    THEN "after_possible_effect"
                    ELSE IF s.oldAccepted
                    THEN "after_native_accept"
                    ELSE IF s.stage = "dispatch_started"
                    THEN "after_dispatch_start"
                    ELSE IF s.stage = "reserved"
                    THEN "after_reservation"
                    ELSE "before_reservation"]

Restart ==
    /\ s.processStatus = "crashed"
    /\ s.bootEpoch < MaxEpoch
    /\ s' = [s EXCEPT
                !.processStatus = "running",
                !.bootEpoch = @ + 1,
                !.crashCut =
                    IF s.crashCut = "permit_consumed_before_effect"
                    THEN "after_recovery_restart"
                    ELSE "after_restart_before_fence"]

FenceControllerQueue ==
    /\ s.processStatus = "running"
    /\ s.bootEpoch > 1
    /\ s.queueFenceBootEpoch < s.bootEpoch
    /\ s.controllerEpoch < MaxEpoch
    /\ s' = [s EXCEPT
                !.queueFenceBootEpoch = s.bootEpoch,
                !.controllerEpoch = @ + 1,
                !.crashCut = "after_controller_queue_fence"]

FenceDownstreamExecution ==
    /\ s.processStatus = "running"
    /\ s.bootEpoch > 1
    /\ s.queueFenceBootEpoch = s.bootEpoch
    /\ s.downstreamFenceBootEpoch < s.bootEpoch
    /\ s' = [s EXCEPT
                !.downstreamFenceBootEpoch = s.bootEpoch,
                !.oldPhysicalOutcome =
                    IF s.commandPending /\ s.commandOwnerAttempt = OldAttemptId
                    THEN "no_effect"
                    ELSE @,
                !.recoveryPhysicalOutcome =
                    IF s.commandPending /\ s.commandOwnerAttempt = RecoveryAttemptId
                    THEN "no_effect"
                    ELSE @,
                !.commandPending = FALSE,
                !.commandOwnerAttempt = "none",
                !.crashCut = "after_downstream_execution_fence"]

ResolvePersistence ==
    /\ s.processStatus = "running"
    /\ s.bootEpoch > 1
    /\ s.persistStatus = "indeterminate_commit"
    /\ ~s.storeCallPending
    /\ s' = [s EXCEPT
                !.persistStatus =
                    IF s.storeRecord
                    THEN "committed"
                    ELSE "definitely_not_committed"]

ObserveOldEffect ==
    /\ s.processStatus = "running"
    /\ s.bootEpoch > 1
    /\ s.downstreamFenceBootEpoch = s.bootEpoch
    /\ s.observerIndependent
    /\ s.oldPhysicalOutcome \in {"applied", "no_effect", "partial"}
    /\ s.effectEvidence = "none"
    /\ s' = [s EXCEPT !.effectEvidence = s.oldPhysicalOutcome]

ObserveDisputedEffect ==
    /\ s.processStatus = "running"
    /\ s.bootEpoch > 1
    /\ s.downstreamFenceBootEpoch = s.bootEpoch
    /\ s.oldPhysicalOutcome # "none"
    /\ s.effectEvidence = "none"
    /\ s' = [s EXCEPT
                !.effectEvidence = "disputed",
                !.observerIndependent = FALSE]

ResolveMission ==
    /\ s.processStatus = "running"
    /\ s.effectEvidence \in {"applied", "no_effect", "partial", "disputed"}
    /\ s.missionResolution = "none"
    /\ s' = [s EXCEPT !.missionResolution = s.effectEvidence]

IssueRecoveryPermit ==
    /\ s.processStatus = "running"
    /\ s.bootEpoch > 1
    /\ s.queueFenceBootEpoch = s.bootEpoch
    /\ s.downstreamFenceBootEpoch = s.bootEpoch
    /\ s.oldPhysicalOutcome = "no_effect"
    /\ s.effectEvidence = "no_effect"
    /\ s.missionResolution = "no_effect"
    /\ s.persistStatus \in {"committed", "definitely_not_committed"}
    /\ s.m1Current
    /\ s.m2Current
    /\ s.recoveryPermit = "absent"
    /\ ~s.recoveryAccepted
    /\ s' = [s EXCEPT !.recoveryPermit = "issued"]

CreateFreshTicket ==
    /\ s.processStatus = "running"
    /\ s.recoveryPermit = "issued"
    /\ s.queueFenceBootEpoch = s.bootEpoch
    /\ s.downstreamFenceBootEpoch = s.bootEpoch
    /\ s.m1Current
    /\ s.m2Current
    /\ s' = [s EXCEPT
                !.ticketStatus = "fresh",
                !.ticketBootEpoch = s.bootEpoch,
                !.ticketControllerEpoch = s.controllerEpoch,
                !.ticketAttempt = RecoveryAttemptId,
                !.ticketBindingValid = TRUE,
                !.liveBindingValid = TRUE]

RevokeM1 ==
    /\ s.m1Current
    /\ s' = [s EXCEPT !.m1Current = FALSE]

RevokeM2 ==
    /\ s.m2Current
    /\ s' = [s EXCEPT !.m2Current = FALSE]

ChangeLiveBinding ==
    /\ s.ticketStatus = "fresh"
    /\ s.liveBindingValid
    /\ s' = [s EXCEPT !.liveBindingValid = FALSE]

NativeAcceptRecovery ==
    /\ s.processStatus = "running"
    /\ s.recoveryPermit = "issued"
    /\ s.ticketStatus = "fresh"
    /\ s.ticketAttempt = RecoveryAttemptId
    /\ s.ticketBootEpoch = s.bootEpoch
    /\ s.ticketControllerEpoch = s.controllerEpoch
    /\ s.queueFenceBootEpoch = s.bootEpoch
    /\ s.downstreamFenceBootEpoch = s.bootEpoch
    /\ s.effectEvidence = "no_effect"
    /\ s.missionResolution = "no_effect"
    /\ s.persistStatus \in {"committed", "definitely_not_committed"}
    /\ s.m1Current
    /\ s.m2Current
    /\ s.ticketBindingValid
    /\ s.liveBindingValid
    /\ ~s.recoveryAccepted
    /\ ~s.commandPending
    /\ s' = [s EXCEPT
                !.recoveryPermit = "consumed",
                !.permitUseCount = @ + 1,
                !.ticketStatus = "consumed",
                !.recoveryAccepted = TRUE,
                !.admittedFreshBoot = (s.ticketBootEpoch = s.bootEpoch),
                !.admittedFreshController = (s.ticketControllerEpoch = s.controllerEpoch),
                !.admittedControllerFence = (s.queueFenceBootEpoch = s.bootEpoch),
                !.admittedDownstreamFence = (s.downstreamFenceBootEpoch = s.bootEpoch),
                !.admittedM1 = s.m1Current,
                !.admittedM2 = s.m2Current,
                !.admittedBinding = (s.ticketBindingValid /\ s.liveBindingValid),
                !.commandPending = TRUE,
                !.commandOwnerAttempt = RecoveryAttemptId,
                !.downstreamFenceBootEpoch = 0,
                !.crashCut = "permit_consumed_before_effect"]

RecoveryDeviceApplies ==
    /\ s.commandPending
    /\ s.commandOwnerAttempt = RecoveryAttemptId
    /\ s.downstreamFenceBootEpoch # s.bootEpoch
    /\ s.recoveryPhysicalOutcome = "none"
    /\ s' = [s EXCEPT
                !.recoveryPhysicalOutcome = "applied",
                !.appliedEffectCount = @ + 1,
                !.commandPending = FALSE,
                !.commandOwnerAttempt = "none"]

RecoveryDeviceDoesNotApply ==
    /\ s.commandPending
    /\ s.commandOwnerAttempt = RecoveryAttemptId
    /\ s.downstreamFenceBootEpoch # s.bootEpoch
    /\ s.recoveryPhysicalOutcome = "none"
    /\ s' = [s EXCEPT
                !.recoveryPhysicalOutcome = "no_effect",
                !.commandPending = FALSE,
                !.commandOwnerAttempt = "none"]

RecoveryDevicePartiallyApplies ==
    /\ s.commandPending
    /\ s.commandOwnerAttempt = RecoveryAttemptId
    /\ s.downstreamFenceBootEpoch # s.bootEpoch
    /\ s.recoveryPhysicalOutcome = "none"
    /\ s' = [s EXCEPT
                !.recoveryPhysicalOutcome = "partial",
                !.commandPending = FALSE,
                !.commandOwnerAttempt = "none"]

NaiveRetry ==
    /\ NaiveRetryEnabled
    /\ s.processStatus = "running"
    /\ s.bootEpoch > 1
    /\ s.oldAccepted
    /\ s.oldPhysicalOutcome = "applied"
    /\ s' = [s EXCEPT
                !.appliedEffectCount = @ + 1,
                !.naiveReplayCount = @ + 1]

Next ==
    \/ ReserveAttempt
    \/ MarkDispatchStarted
    \/ NativeAcceptOld
    \/ OldDeviceApplies
    \/ OldDeviceDoesNotApply
    \/ OldDevicePartiallyApplies
    \/ BeginEffectRecordCommit
    \/ CommitEffectRecord
    \/ AbortEffectRecord
    \/ AcknowledgeStoreCommit
    \/ AcknowledgeStoreNoCommit
    \/ Crash
    \/ Restart
    \/ FenceControllerQueue
    \/ FenceDownstreamExecution
    \/ ResolvePersistence
    \/ ObserveOldEffect
    \/ ObserveDisputedEffect
    \/ ResolveMission
    \/ IssueRecoveryPermit
    \/ CreateFreshTicket
    \/ RevokeM1
    \/ RevokeM2
    \/ ChangeLiveBinding
    \/ NativeAcceptRecovery
    \/ RecoveryDeviceApplies
    \/ RecoveryDeviceDoesNotApply
    \/ RecoveryDevicePartiallyApplies
    \/ (NaiveRetryEnabled /\ NaiveRetry)

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ s.processStatus \in {"running", "crashed"}
    /\ s.bootEpoch \in 1..MaxEpoch
    /\ s.controllerEpoch \in 1..MaxEpoch
    /\ s.stage \in {"unreserved", "reserved", "dispatch_started", "accepted"}
    /\ s.reservationCommitted \in BOOLEAN
    /\ s.oldTicketBootEpoch \in 1..MaxEpoch
    /\ s.oldTicketControllerEpoch \in 1..MaxEpoch
    /\ s.oldAccepted \in BOOLEAN
    /\ s.oldAcceptedBootEpoch \in 0..MaxEpoch
    /\ s.oldAcceptedControllerEpoch \in 0..MaxEpoch
    /\ s.commandPending \in BOOLEAN
    /\ s.commandOwnerAttempt \in {"none", OldAttemptId, RecoveryAttemptId}
    /\ s.oldPhysicalOutcome \in {"none", "applied", "no_effect", "partial"}
    /\ s.recoveryPhysicalOutcome \in {"none", "applied", "no_effect", "partial"}
    /\ s.appliedEffectCount \in 0..2
    /\ s.persistStatus \in
        {"definitely_not_committed", "in_flight", "committed", "indeterminate_commit"}
    /\ s.storeRecord \in BOOLEAN
    /\ s.storeCallPending \in BOOLEAN
    /\ s.effectEvidence \in {"none", "applied", "no_effect", "partial", "disputed"}
    /\ s.missionResolution \in {"none", "applied", "no_effect", "partial", "disputed"}
    /\ s.observerIndependent \in BOOLEAN
    /\ s.queueFenceBootEpoch \in 0..MaxEpoch
    /\ s.downstreamFenceBootEpoch \in 0..MaxEpoch
    /\ s.recoveryPermit \in {"absent", "issued", "consumed"}
    /\ s.permitUseCount \in 0..2
    /\ s.ticketStatus \in {"old", "fresh", "consumed"}
    /\ s.ticketBootEpoch \in 1..MaxEpoch
    /\ s.ticketControllerEpoch \in 1..MaxEpoch
    /\ s.ticketAttempt \in {OldAttemptId, RecoveryAttemptId}
    /\ s.m1Current \in BOOLEAN
    /\ s.m2Current \in BOOLEAN
    /\ s.ticketBindingValid \in BOOLEAN
    /\ s.liveBindingValid \in BOOLEAN
    /\ s.recoveryAccepted \in BOOLEAN
    /\ s.admittedFreshBoot \in BOOLEAN
    /\ s.admittedFreshController \in BOOLEAN
    /\ s.admittedControllerFence \in BOOLEAN
    /\ s.admittedDownstreamFence \in BOOLEAN
    /\ s.admittedM1 \in BOOLEAN
    /\ s.admittedM2 \in BOOLEAN
    /\ s.admittedBinding \in BOOLEAN
    /\ s.naiveReplayCount \in 0..2
    /\ s.crashCut \in
        {"none", "before_reservation", "after_reservation",
         "after_dispatch_start", "after_native_accept", "after_possible_effect",
         "after_restart_before_fence", "after_controller_queue_fence",
         "after_downstream_execution_fence", "permit_consumed_before_effect",
         "after_recovery_restart"}

NoDuplicatePhysicalEffects ==
    s.appliedEffectCount <= 1

NoRecoveryFromUnknownPartialOrDisputed ==
    s.effectEvidence \in {"none", "partial", "disputed"} =>
        s.recoveryPermit = "absent" /\ ~s.recoveryAccepted

NoRecoveryFromApplied ==
    s.effectEvidence = "applied" =>
        s.recoveryPermit = "absent" /\ ~s.recoveryAccepted

IndeterminatePersistenceBlocksRecovery ==
    s.persistStatus = "indeterminate_commit" =>
        s.recoveryPermit = "absent" /\ ~s.recoveryAccepted

RecoveryRequiresExactNoEffect ==
    s.recoveryAccepted =>
        /\ s.oldPhysicalOutcome = "no_effect"
        /\ s.effectEvidence = "no_effect"
        /\ s.missionResolution = "no_effect"
        /\ s.persistStatus \in {"committed", "definitely_not_committed"}

RecoveryAdmissionWasCurrentAndFenced ==
    s.recoveryAccepted =>
        /\ s.admittedFreshBoot
        /\ s.admittedFreshController
        /\ s.admittedControllerFence
        /\ s.admittedDownstreamFence
        /\ s.admittedM1
        /\ s.admittedM2
        /\ s.admittedBinding

RecoveryPermitIsOneShot ==
    /\ s.permitUseCount <= 1
    /\ (s.recoveryPermit = "consumed" <=> s.permitUseCount = 1)
    /\ (s.recoveryAccepted <=> s.recoveryPermit = "consumed")

OldCommandWasAcceptedOnlyInItsOriginalEpoch ==
    s.oldAccepted =>
        /\ s.oldAcceptedBootEpoch = 1
        /\ s.oldAcceptedControllerEpoch = 1

DownstreamFenceClosesPendingCommand ==
    s.downstreamFenceBootEpoch = s.bootEpoch => ~s.commandPending

DownstreamFenceRequiresControllerFence ==
    s.downstreamFenceBootEpoch = s.bootEpoch =>
        s.queueFenceBootEpoch = s.bootEpoch

ReservationPrecedesNativeAcceptance ==
    s.oldAccepted => s.reservationCommitted

StoreVerdictMatchesDurableRecord ==
    /\ (s.persistStatus = "committed" => s.storeRecord)
    /\ (s.persistStatus = "definitely_not_committed" => ~s.storeRecord)
