---- MODULE RestartSafeEffectContinuityMultiAttempt ----
EXTENDS Naturals, TLC

CONSTANTS MaxAttempts, MaxEpoch, UnsafeRetryEnabled

ASSUME MaxAttempts \in Nat /\ MaxAttempts >= 3
ASSUME MaxEpoch \in Nat /\ MaxEpoch >= MaxAttempts
ASSUME UnsafeRetryEnabled \in BOOLEAN

Attempts == 1..MaxAttempts
ResolvedPersistence == {"committed", "definitely_not_committed"}
PhysicalOutcomes == {"none", "applied", "no_effect", "partial", "unknown", "disputed"}
AttemptStatuses == {"unreserved", "reserved", "dispatch_started", "accepted", "reconciled"}

LogicalEffectId == "logical-effect-1"

InitialAttempt(i) ==
    [ logicalEffectId |-> LogicalEffectId,
      status |-> "unreserved",
      parent |-> IF i = 1 THEN 0 ELSE i - 1,
      reservationCommitted |-> FALSE,
      dispatchStarted |-> FALSE,
      authorizedBootEpoch |-> 0,
      authorizedControllerEpoch |-> 0,
      commandPending |-> FALSE,
      commandOwner |-> 0,
      physicalOutcome |-> "none",
      physicalMutation |-> FALSE,
      persistence |-> "definitely_not_committed",
      storeRecord |-> FALSE,
      storeCallPending |-> FALSE,
      physicalObserverQualified |-> FALSE,
      downstreamObserverQualified |-> FALSE,
      observerProofsIndependent |-> FALSE,
      observerCommonCause |-> FALSE,
      observerSelfReport |-> FALSE,
      downstreamRetention |-> FALSE,
      missionResolution |-> "none",
      controllerFenceBootEpoch |-> 0,
      controllerFenceControllerEpoch |-> 0,
      downstreamFenceBootEpoch |-> 0,
      downstreamFenceControllerEpoch |-> 0,
      admittedFreshBoot |-> FALSE,
      admittedFreshController |-> FALSE,
      admittedControllerFence |-> FALSE,
      admittedDownstreamFence |-> FALSE,
      admittedM1 |-> FALSE,
      admittedM2 |-> FALSE,
      admittedBinding |-> FALSE ]

vars == <<s>>

Init ==
    s =
        [ processStatus |-> "running",
          bootEpoch |-> 1,
          controllerEpoch |-> 1,
          activeAttempt |-> 1,
          attempts |-> [i \in Attempts |-> InitialAttempt(i)],
          physicalMutationCount |-> 0,
          recoveryPermit |-> "absent",
          permitForAttempt |-> 0,
          permitUseCount |-> 0,
          ticketStatus |-> "absent",
          ticketAttempt |-> 0,
          ticketBootEpoch |-> 0,
          ticketControllerEpoch |-> 0,
          m1Current |-> TRUE,
          m2Current |-> TRUE,
          ticketBindingValid |-> TRUE,
          liveBindingValid |-> TRUE,
          unsafeRetryCount |-> 0,
          crashCut |-> "none" ]

ReserveInitialAttempt ==
    /\ s.processStatus = "running"
    /\ s.activeAttempt = 1
    /\ s.attempts[1].status = "unreserved"
    /\ s' = [s EXCEPT
                !.attempts[1].status = "reserved",
                !.attempts[1].reservationCommitted = TRUE,
                !.attempts[1].authorizedBootEpoch = s.bootEpoch,
                !.attempts[1].authorizedControllerEpoch = s.controllerEpoch,
                !.crashCut = "after_attempt_1_reservation"]

MarkDispatchStarted ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "reserved"
        /\ s.attempts[i].reservationCommitted
        /\ s' = [s EXCEPT
                    !.attempts[i].status = "dispatch_started",
                    !.attempts[i].dispatchStarted = TRUE,
                    !.crashCut = IF i = 2
                                    THEN "attempt_2_after_dispatch_start"
                                    ELSE "after_dispatch_start"]

NativeAcceptCurrentAttempt ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ i \in Attempts
        /\ s.attempts[i].status = "dispatch_started"
        /\ s.attempts[i].reservationCommitted
        /\ s.attempts[i].dispatchStarted
        /\ s.attempts[i].authorizedBootEpoch = s.bootEpoch
        /\ s.attempts[i].authorizedControllerEpoch = s.controllerEpoch
        /\ ~s.attempts[i].commandPending
        /\ (IF i = 1 THEN TRUE ELSE
                /\ s.recoveryPermit = "issued"
                /\ s.permitForAttempt = i
                /\ s.ticketStatus = "fresh"
                /\ s.ticketAttempt = i
                /\ s.ticketBootEpoch = s.bootEpoch
                /\ s.ticketControllerEpoch = s.controllerEpoch
                /\ s.attempts[i - 1].status = "reconciled"
                /\ s.attempts[i - 1].controllerFenceBootEpoch = s.bootEpoch
                /\ s.attempts[i - 1].controllerFenceControllerEpoch = s.controllerEpoch
                /\ s.attempts[i - 1].downstreamFenceBootEpoch = s.bootEpoch
                /\ s.attempts[i - 1].downstreamFenceControllerEpoch = s.controllerEpoch
                /\ s.m1Current
                /\ s.m2Current
                /\ s.ticketBindingValid
                /\ s.liveBindingValid)
        /\ s' = [s EXCEPT
                    !.attempts[i].status = "accepted",
                    !.attempts[i].commandPending = TRUE,
                    !.attempts[i].commandOwner = i,
                    !.attempts[i].admittedFreshBoot =
                        IF i = 1 THEN TRUE ELSE s.ticketBootEpoch = s.bootEpoch,
                    !.attempts[i].admittedFreshController =
                        IF i = 1 THEN TRUE ELSE s.ticketControllerEpoch = s.controllerEpoch,
                    !.attempts[i].admittedControllerFence =
                        IF i = 1 THEN TRUE
                        ELSE s.attempts[i - 1].controllerFenceBootEpoch = s.bootEpoch
                             /\ s.attempts[i - 1].controllerFenceControllerEpoch = s.controllerEpoch,
                    !.attempts[i].admittedDownstreamFence =
                        IF i = 1 THEN TRUE
                        ELSE s.attempts[i - 1].downstreamFenceBootEpoch = s.bootEpoch
                             /\ s.attempts[i - 1].downstreamFenceControllerEpoch = s.controllerEpoch,
                    !.attempts[i].admittedM1 = IF i = 1 THEN TRUE ELSE s.m1Current,
                    !.attempts[i].admittedM2 = IF i = 1 THEN TRUE ELSE s.m2Current,
                    !.attempts[i].admittedBinding =
                        IF i = 1 THEN TRUE ELSE s.ticketBindingValid /\ s.liveBindingValid,
                    !.recoveryPermit = IF i = 1 THEN "absent" ELSE "consumed",
                    !.permitUseCount = @ + (IF i = 1 THEN 0 ELSE 1),
                    !.ticketStatus = IF i = 1 THEN "absent" ELSE "consumed",
                    !.crashCut = IF i = 1
                                    THEN "attempt_1_native_accept_before_effect"
                                    ELSE IF i = 2
                                    THEN "attempt_2_permit_consumed_before_effect"
                                    ELSE "attempt_3_permit_consumed_before_effect"]

DeviceApplies ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "accepted"
        /\ s.attempts[i].commandPending
        /\ s.attempts[i].commandOwner = i
        /\ s.attempts[i].physicalOutcome = "none"
        /\ s' = [s EXCEPT
                    !.attempts[i].physicalOutcome = "applied",
                    !.attempts[i].physicalMutation = TRUE,
                    !.attempts[i].commandPending = FALSE,
                    !.attempts[i].commandOwner = 0,
                    !.physicalMutationCount = @ + 1,
                    !.crashCut = IF i = 2
                                    THEN "attempt_2_after_possible_effect"
                                    ELSE "after_possible_effect"]

DeviceDoesNotApply ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "accepted"
        /\ s.attempts[i].commandPending
        /\ s.attempts[i].commandOwner = i
        /\ s.attempts[i].physicalOutcome = "none"
        /\ s' = [s EXCEPT
                    !.attempts[i].physicalOutcome = "no_effect",
                    !.attempts[i].commandPending = FALSE,
                    !.attempts[i].commandOwner = 0]

DevicePartiallyApplies ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "accepted"
        /\ s.attempts[i].commandPending
        /\ s.attempts[i].commandOwner = i
        /\ s.attempts[i].physicalOutcome = "none"
        /\ s' = [s EXCEPT
                    !.attempts[i].physicalOutcome = "partial",
                    !.attempts[i].physicalMutation = TRUE,
                    !.attempts[i].commandPending = FALSE,
                    !.attempts[i].commandOwner = 0,
                    !.physicalMutationCount = @ + 1]

DeviceOutcomeUnknown ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "accepted"
        /\ s.attempts[i].commandPending
        /\ s.attempts[i].physicalOutcome = "none"
        /\ s' = [s EXCEPT
                    !.attempts[i].physicalOutcome = "unknown",
                    !.attempts[i].commandPending = FALSE,
                    !.attempts[i].commandOwner = 0]

BeginEffectRecordCommit ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].physicalOutcome # "none"
        /\ s.attempts[i].persistence = "definitely_not_committed"
        /\ ~s.attempts[i].storeCallPending
        /\ ~s.attempts[i].storeRecord
        /\ s' = [s EXCEPT
                    !.attempts[i].persistence = "in_flight",
                    !.attempts[i].storeCallPending = TRUE]

CommitEffectRecord ==
    LET i == s.activeAttempt IN
        /\ s.attempts[i].storeCallPending
        /\ ~s.attempts[i].storeRecord
        /\ s' = [s EXCEPT
                    !.attempts[i].storeRecord = TRUE,
                    !.attempts[i].storeCallPending = FALSE]

AbortEffectRecord ==
    LET i == s.activeAttempt IN
        /\ s.attempts[i].storeCallPending
        /\ ~s.attempts[i].storeRecord
        /\ s' = [s EXCEPT !.attempts[i].storeCallPending = FALSE]

AcknowledgeStoreCommit ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].persistence = "in_flight"
        /\ s.attempts[i].storeRecord
        /\ ~s.attempts[i].storeCallPending
        /\ s' = [s EXCEPT !.attempts[i].persistence = "committed"]

AcknowledgeStoreNoCommit ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].persistence = "in_flight"
        /\ ~s.attempts[i].storeRecord
        /\ ~s.attempts[i].storeCallPending
        /\ s' = [s EXCEPT !.attempts[i].persistence = "definitely_not_committed"]

Crash ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "accepted"
        /\ s' = [s EXCEPT
                    !.processStatus = "crashed",
                    !.attempts[i].persistence =
                        IF @ = "in_flight" THEN "indeterminate_commit" ELSE @,
                    !.attempts[i].storeCallPending = FALSE,
                    !.crashCut =
                        IF i = 2 /\ s.attempts[i].physicalOutcome = "none"
                        THEN "attempt_2_crash_after_accept_before_effect"
                        ELSE IF i = 2
                        THEN "attempt_2_crash_after_possible_effect"
                        ELSE IF s.attempts[i].physicalOutcome = "none"
                        THEN "crash_after_accept_before_effect"
                        ELSE "crash_after_possible_effect"]

Restart ==
    /\ s.processStatus = "crashed"
    /\ s.bootEpoch < MaxEpoch
    /\ s' = [s EXCEPT
                !.processStatus = "running",
                !.bootEpoch = @ + 1,
                !.crashCut = "after_restart_before_fence"]

FenceControllerQueue ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.bootEpoch > s.attempts[i].authorizedBootEpoch
        /\ s.controllerEpoch < MaxEpoch
        /\ s.attempts[i].controllerFenceBootEpoch < s.bootEpoch
        /\ s' = [s EXCEPT
                    !.controllerEpoch = @ + 1,
                    !.attempts[i].controllerFenceBootEpoch = s.bootEpoch,
                    !.attempts[i].controllerFenceControllerEpoch = s.controllerEpoch + 1,
                    !.crashCut = "after_controller_queue_fence"]

FenceDownstreamExecution ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].controllerFenceBootEpoch = s.bootEpoch
        /\ s.attempts[i].controllerFenceControllerEpoch = s.controllerEpoch
        /\ s.attempts[i].downstreamFenceBootEpoch < s.bootEpoch
        /\ ~s.attempts[i].downstreamRetention
        /\ s' = [s EXCEPT
                    !.attempts[i].downstreamFenceBootEpoch = s.bootEpoch,
                    !.attempts[i].downstreamFenceControllerEpoch = s.controllerEpoch,
                    !.attempts[i].physicalOutcome =
                        IF s.attempts[i].commandPending /\ s.attempts[i].physicalOutcome = "none"
                        THEN "no_effect"
                        ELSE s.attempts[i].physicalOutcome,
                    !.attempts[i].commandPending = FALSE,
                    !.attempts[i].commandOwner = 0,
                    !.crashCut = "after_downstream_execution_fence"]

ResolvePersistence ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].persistence = "indeterminate_commit"
        /\ ~s.attempts[i].storeCallPending
        /\ s' = [s EXCEPT
                    !.attempts[i].persistence =
                        IF s.attempts[i].storeRecord
                        THEN "committed"
                        ELSE "definitely_not_committed"]

InjectObserverCommonCauseFailure ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "accepted"
        /\ ~s.attempts[i].physicalObserverQualified
        /\ ~s.attempts[i].downstreamObserverQualified
        /\ s' = [s EXCEPT !.attempts[i].observerCommonCause = TRUE]

InjectSubjectSelfReport ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "accepted"
        /\ ~s.attempts[i].physicalObserverQualified
        /\ ~s.attempts[i].downstreamObserverQualified
        /\ s' = [s EXCEPT !.attempts[i].observerSelfReport = TRUE]

InjectDownstreamRetention ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "accepted"
        /\ s.attempts[i].downstreamFenceBootEpoch = 0
        /\ s' = [s EXCEPT !.attempts[i].downstreamRetention = TRUE]

ClearDownstreamRetention ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.bootEpoch > s.attempts[i].authorizedBootEpoch
        /\ s.attempts[i].downstreamRetention
        /\ s' = [s EXCEPT !.attempts[i].downstreamRetention = FALSE]

QualifyPhysicalOutcomeObserver ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].physicalOutcome \in {"applied", "no_effect", "partial", "unknown", "disputed"}
        /\ s.attempts[i].downstreamFenceBootEpoch = s.bootEpoch
        /\ s.attempts[i].downstreamFenceControllerEpoch = s.controllerEpoch
        /\ ~s.attempts[i].observerCommonCause
        /\ ~s.attempts[i].observerSelfReport
        /\ ~s.attempts[i].physicalObserverQualified
        /\ s' = [s EXCEPT !.attempts[i].physicalObserverQualified = TRUE]

QualifyDownstreamQuiescenceObserver ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].downstreamFenceBootEpoch = s.bootEpoch
        /\ s.attempts[i].downstreamFenceControllerEpoch = s.controllerEpoch
        /\ ~s.attempts[i].commandPending
        /\ ~s.attempts[i].observerCommonCause
        /\ ~s.attempts[i].observerSelfReport
        /\ ~s.attempts[i].downstreamObserverQualified
        /\ s' = [s EXCEPT !.attempts[i].downstreamObserverQualified = TRUE]

QualifyObserverIndependence ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].physicalObserverQualified
        /\ s.attempts[i].downstreamObserverQualified
        /\ ~s.attempts[i].observerCommonCause
        /\ ~s.attempts[i].observerSelfReport
        /\ ~s.attempts[i].observerProofsIndependent
        /\ s' = [s EXCEPT !.attempts[i].observerProofsIndependent = TRUE]

ResolveMission ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].physicalObserverQualified
        /\ s.attempts[i].missionResolution = "none"
        /\ s' = [s EXCEPT
                    !.attempts[i].missionResolution =
                        CASE s.attempts[i].physicalOutcome = "no_effect" -> "no_effect"
                          [] s.attempts[i].physicalOutcome = "applied" -> "effect_predicate_satisfied"
                          [] OTHER -> "unresolved"]

ReconcileNoEffectAttempt ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ s.attempts[i].status = "accepted"
        /\ s.attempts[i].physicalOutcome = "no_effect"
        /\ s.attempts[i].physicalObserverQualified
        /\ s.attempts[i].downstreamObserverQualified
        /\ s.attempts[i].observerProofsIndependent
        /\ s.attempts[i].missionResolution = "no_effect"
        /\ s.attempts[i].persistence \in ResolvedPersistence
        /\ ~s.attempts[i].storeCallPending
        /\ ~s.attempts[i].commandPending
        /\ s.attempts[i].controllerFenceBootEpoch = s.bootEpoch
        /\ s.attempts[i].controllerFenceControllerEpoch = s.controllerEpoch
        /\ s.attempts[i].downstreamFenceBootEpoch = s.bootEpoch
        /\ s.attempts[i].downstreamFenceControllerEpoch = s.controllerEpoch
        /\ s.m1Current
        /\ s.m2Current
        /\ s' = [s EXCEPT !.attempts[i].status = "reconciled"]

IssueRecoveryPermit ==
    LET i == s.activeAttempt IN
        /\ s.processStatus = "running"
        /\ i < MaxAttempts
        /\ s.attempts[i].status = "reconciled"
        /\ s.recoveryPermit \in {"absent", "consumed"}
        /\ s.permitUseCount < MaxAttempts - 1
        /\ s' = [s EXCEPT
                    !.recoveryPermit = "issued",
                    !.permitForAttempt = i + 1,
                    !.ticketStatus = "absent",
                    !.ticketAttempt = 0,
                    !.ticketBootEpoch = 0,
                    !.ticketControllerEpoch = 0]

CreateFreshTicket ==
    /\ s.processStatus = "running"
    /\ s.recoveryPermit = "issued"
    /\ s.permitForAttempt \in Attempts
    /\ s.attempts[s.permitForAttempt - 1].status = "reconciled"
    /\ s' = [s EXCEPT
                !.ticketStatus = "fresh",
                !.ticketAttempt = s.permitForAttempt,
                !.ticketBootEpoch = s.bootEpoch,
                !.ticketControllerEpoch = s.controllerEpoch,
                !.ticketBindingValid = TRUE,
                !.liveBindingValid = TRUE]

ReserveNextAttempt ==
    LET i == s.permitForAttempt IN
        /\ s.processStatus = "running"
        /\ s.recoveryPermit = "issued"
        /\ i \in 2..MaxAttempts
        /\ s.ticketStatus = "fresh"
        /\ s.ticketAttempt = i
        /\ s.ticketBootEpoch = s.bootEpoch
        /\ s.ticketControllerEpoch = s.controllerEpoch
        /\ s.attempts[i].status = "unreserved"
        /\ s.attempts[i - 1].status = "reconciled"
        /\ s' = [s EXCEPT
                    !.activeAttempt = i,
                    !.attempts[i].status = "reserved",
                    !.attempts[i].parent = i - 1,
                    !.attempts[i].reservationCommitted = TRUE,
                    !.attempts[i].authorizedBootEpoch = s.bootEpoch,
                    !.attempts[i].authorizedControllerEpoch = s.controllerEpoch]

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

UnsafeRetryBypass ==
    /\ UnsafeRetryEnabled
    /\ s.processStatus = "running"
    /\ s.physicalMutationCount = 1
    /\ s.unsafeRetryCount = 0
    /\ s' = [s EXCEPT
                !.physicalMutationCount = @ + 1,
                !.unsafeRetryCount = @ + 1,
                !.crashCut = "deliberately_unsafe_retry"]

Next ==
    \/ ReserveInitialAttempt
    \/ MarkDispatchStarted
    \/ NativeAcceptCurrentAttempt
    \/ DeviceApplies
    \/ DeviceDoesNotApply
    \/ DevicePartiallyApplies
    \/ DeviceOutcomeUnknown
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
    \/ InjectObserverCommonCauseFailure
    \/ InjectSubjectSelfReport
    \/ InjectDownstreamRetention
    \/ ClearDownstreamRetention
    \/ QualifyPhysicalOutcomeObserver
    \/ QualifyDownstreamQuiescenceObserver
    \/ QualifyObserverIndependence
    \/ ResolveMission
    \/ ReconcileNoEffectAttempt
    \/ IssueRecoveryPermit
    \/ CreateFreshTicket
    \/ ReserveNextAttempt
    \/ RevokeM1
    \/ RevokeM2
    \/ ChangeLiveBinding
    \/ UnsafeRetryBypass

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ s.processStatus \in {"running", "crashed"}
    /\ s.bootEpoch \in 1..MaxEpoch
    /\ s.controllerEpoch \in 1..MaxEpoch
    /\ s.activeAttempt \in Attempts
    /\ s.attempts \in [Attempts -> [logicalEffectId : {LogicalEffectId},
                                  status : AttemptStatuses,
                                  parent : 0..MaxAttempts,
                                  reservationCommitted : BOOLEAN,
                                  dispatchStarted : BOOLEAN,
                                  authorizedBootEpoch : 0..MaxEpoch,
                                  authorizedControllerEpoch : 0..MaxEpoch,
                                  commandPending : BOOLEAN,
                                  commandOwner : 0..MaxAttempts,
                                  physicalOutcome : PhysicalOutcomes,
                                  physicalMutation : BOOLEAN,
                                  persistence : {"definitely_not_committed", "in_flight", "committed", "indeterminate_commit"},
                                  storeRecord : BOOLEAN,
                                  storeCallPending : BOOLEAN,
                                  physicalObserverQualified : BOOLEAN,
                                  downstreamObserverQualified : BOOLEAN,
                                  observerProofsIndependent : BOOLEAN,
                                  observerCommonCause : BOOLEAN,
                                  observerSelfReport : BOOLEAN,
                                  downstreamRetention : BOOLEAN,
                                  missionResolution : {"none", "no_effect", "effect_predicate_satisfied", "unresolved"},
                                  controllerFenceBootEpoch : 0..MaxEpoch,
                                  controllerFenceControllerEpoch : 0..MaxEpoch,
                                  downstreamFenceBootEpoch : 0..MaxEpoch,
                                  downstreamFenceControllerEpoch : 0..MaxEpoch,
                                  admittedFreshBoot : BOOLEAN,
                                  admittedFreshController : BOOLEAN,
                                  admittedControllerFence : BOOLEAN,
                                  admittedDownstreamFence : BOOLEAN,
                                  admittedM1 : BOOLEAN,
                                  admittedM2 : BOOLEAN,
                                  admittedBinding : BOOLEAN]]
    /\ s.physicalMutationCount \in 0..2
    /\ s.recoveryPermit \in {"absent", "issued", "consumed"}
    /\ s.permitForAttempt \in 0..MaxAttempts
    /\ s.permitUseCount \in 0..MaxAttempts
    /\ s.ticketStatus \in {"absent", "fresh", "consumed"}
    /\ s.ticketAttempt \in 0..MaxAttempts
    /\ s.ticketBootEpoch \in 0..MaxEpoch
    /\ s.ticketControllerEpoch \in 0..MaxEpoch
    /\ s.m1Current \in BOOLEAN
    /\ s.m2Current \in BOOLEAN
    /\ s.ticketBindingValid \in BOOLEAN
    /\ s.liveBindingValid \in BOOLEAN
    /\ s.unsafeRetryCount \in 0..1
    /\ s.crashCut \in
        {"none", "after_attempt_1_reservation", "after_dispatch_start",
         "attempt_2_after_dispatch_start", "attempt_1_native_accept_before_effect",
         "attempt_2_permit_consumed_before_effect", "attempt_3_permit_consumed_before_effect", "after_possible_effect",
         "attempt_2_after_possible_effect", "crash_after_accept_before_effect",
         "attempt_2_crash_after_accept_before_effect", "crash_after_possible_effect",
         "attempt_2_crash_after_possible_effect", "after_restart_before_fence",
         "after_controller_queue_fence", "after_downstream_execution_fence",
         "deliberately_unsafe_retry"}

NoDuplicatePhysicalMutations ==
    s.physicalMutationCount <= 1

AttemptIdentityIsStable ==
    \A i \in Attempts: s.attempts[i].logicalEffectId = LogicalEffectId

AttemptParentLinksAreStable ==
    /\ s.attempts[1].parent = 0
    /\ \A i \in 2..MaxAttempts: s.attempts[i].parent = i - 1

AttemptEpochsIncreaseFor(i) ==
    /\ s.attempts[i].authorizedBootEpoch > s.attempts[i - 1].authorizedBootEpoch
    /\ s.attempts[i].authorizedControllerEpoch > s.attempts[i - 1].authorizedControllerEpoch

AttemptEpochsIncreaseAcrossRecovery ==
    \A i \in 2..MaxAttempts:
        (s.attempts[i].status # "unreserved" => AttemptEpochsIncreaseFor(i))

MissionNoEffectConsistent(i) ==
    /\ s.attempts[i].physicalOutcome = "no_effect"
    /\ s.attempts[i].physicalObserverQualified

MissionAppliedConsistent(i) ==
    /\ s.attempts[i].physicalOutcome = "applied"
    /\ s.attempts[i].physicalObserverQualified

MissionUnresolvedConsistent(i) ==
    /\ s.attempts[i].physicalOutcome \in {"partial", "unknown", "disputed"}
    /\ s.attempts[i].physicalObserverQualified

MissionResolutionMatchesPhysicalOutcome ==
    \A i \in Attempts:
        /\ (s.attempts[i].missionResolution = "no_effect" =>
                MissionNoEffectConsistent(i))
        /\ (s.attempts[i].missionResolution = "effect_predicate_satisfied" =>
                MissionAppliedConsistent(i))
        /\ (s.attempts[i].missionResolution = "unresolved" =>
                MissionUnresolvedConsistent(i))

PermitUseCountMatchesAcceptedChildren ==
    s.permitUseCount =
        Cardinality({i \in 2..MaxAttempts:
            s.attempts[i].status \in {"accepted", "reconciled"}})

ReservationFactsHold(i) ==
    /\ s.attempts[i].reservationCommitted
    /\ s.attempts[i].dispatchStarted
    /\ s.attempts[i].authorizedBootEpoch > 0
    /\ s.attempts[i].authorizedControllerEpoch > 0

ReservationPrecedesAcceptance ==
    \A i \in Attempts:
        (s.attempts[i].status \in {"accepted", "reconciled"} =>
            ReservationFactsHold(i))

RetryParentIsQualified(i) ==
    /\ s.attempts[i - 1].status = "reconciled"
    /\ s.attempts[i - 1].physicalOutcome = "no_effect"
    /\ s.attempts[i - 1].physicalObserverQualified
    /\ s.attempts[i - 1].downstreamObserverQualified
    /\ s.attempts[i - 1].observerProofsIndependent
    /\ s.attempts[i - 1].missionResolution = "no_effect"
    /\ s.attempts[i - 1].persistence \in ResolvedPersistence
    /\ s.attempts[i - 1].controllerFenceBootEpoch = s.attempts[i].authorizedBootEpoch
    /\ s.attempts[i - 1].controllerFenceControllerEpoch = s.attempts[i].authorizedControllerEpoch
    /\ s.attempts[i - 1].downstreamFenceBootEpoch = s.attempts[i].authorizedBootEpoch
    /\ s.attempts[i - 1].downstreamFenceControllerEpoch = s.attempts[i].authorizedControllerEpoch

RetryAdmissionIsQualified ==
    \A i \in 2..MaxAttempts:
        (s.attempts[i].status # "unreserved" => RetryParentIsQualified(i))

CapturedRecoveryAdmissionFactsHold(i) ==
    /\ s.attempts[i].admittedFreshBoot
    /\ s.attempts[i].admittedFreshController
    /\ s.attempts[i].admittedControllerFence
    /\ s.attempts[i].admittedDownstreamFence
    /\ s.attempts[i].admittedM1
    /\ s.attempts[i].admittedM2
    /\ s.attempts[i].admittedBinding

RecoveryAdmissionWasCurrentAndFenced ==
    \A i \in 2..MaxAttempts:
        (s.attempts[i].status \in {"accepted", "reconciled"} =>
            CapturedRecoveryAdmissionFactsHold(i))

NoRetryAfterUnsafePhysicalOutcome ==
    \A i \in 2..MaxAttempts:
        (s.attempts[i].status # "unreserved" =>
            s.attempts[i - 1].physicalOutcome = "no_effect")

NoRetryAfterAmbiguousPersistence ==
    \A i \in 2..MaxAttempts:
        (s.attempts[i].status # "unreserved" =>
            s.attempts[i - 1].persistence \in ResolvedPersistence)

RetryObserverProofsQualified(i) ==
    /\ s.attempts[i - 1].physicalObserverQualified
    /\ s.attempts[i - 1].downstreamObserverQualified
    /\ s.attempts[i - 1].observerProofsIndependent

NoRetryAfterUnqualifiedObserver ==
    \A i \in 2..MaxAttempts:
        (s.attempts[i].status # "unreserved" =>
            RetryObserverProofsQualified(i))

NoRecoveryFromAppliedOrPartial ==
    \A i \in 2..MaxAttempts:
        (s.attempts[i].status # "unreserved" =>
            s.attempts[i - 1].physicalOutcome = "no_effect")

OneShotPermitUseBound ==
    s.permitUseCount <= MaxAttempts - 1

IssuedPermitTargetsNextAttempt ==
    /\ s.permitForAttempt \in 2..MaxAttempts
    /\ s.activeAttempt \in {s.permitForAttempt - 1, s.permitForAttempt}
    /\ s.attempts[s.permitForAttempt].status \in
        {"unreserved", "reserved", "dispatch_started"}
    /\ s.attempts[s.permitForAttempt].parent = s.permitForAttempt - 1

PermitTargetsOnlyNextAttempt ==
    (s.recoveryPermit = "issued" => IssuedPermitTargetsNextAttempt)

CurrentDownstreamFenceClosesPendingCommand(i) ==
    ~s.attempts[i].commandPending

OldQueueFenceClosesPendingCommand ==
    \A i \in Attempts:
        ((s.attempts[i].downstreamFenceBootEpoch = s.bootEpoch
          /\ s.attempts[i].downstreamFenceControllerEpoch = s.controllerEpoch) =>
            CurrentDownstreamFenceClosesPendingCommand(i))

CurrentDownstreamFenceHasControllerFence(i) ==
    /\ s.attempts[i].controllerFenceBootEpoch = s.bootEpoch
    /\ s.attempts[i].controllerFenceControllerEpoch = s.controllerEpoch

DownstreamFenceRequiresControllerFence ==
    \A i \in Attempts:
        ((s.attempts[i].downstreamFenceBootEpoch = s.bootEpoch
          /\ s.attempts[i].downstreamFenceControllerEpoch = s.controllerEpoch) =>
            CurrentDownstreamFenceHasControllerFence(i))

StoreVerdictMatchesDurableRecord ==
    \A i \in Attempts:
        /\ (s.attempts[i].persistence = "committed" => s.attempts[i].storeRecord)
        /\ (s.attempts[i].persistence = "definitely_not_committed" => ~s.attempts[i].storeRecord)

ObserverProofsAreUnfaulted(i) ==
    /\ ~s.attempts[i].observerCommonCause
    /\ ~s.attempts[i].observerSelfReport

NoObserverSelfQualification ==
    \A i \in Attempts:
        ((s.attempts[i].physicalObserverQualified \/
          s.attempts[i].downstreamObserverQualified) =>
            ObserverProofsAreUnfaulted(i))

CommandOwnerMatchesAttempt(i) ==
    s.attempts[i].commandOwner = i

NoCommandOwnedByWrongAttempt ==
    \A i \in Attempts:
        (s.attempts[i].commandPending => CommandOwnerMatchesAttempt(i))

====
