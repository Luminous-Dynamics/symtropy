---- MODULE RestartSafeEffectContinuityTrace ----
EXTENDS RestartSafeEffectContinuity, Sequences, TLC, Json

CONSTANT TraceFile

Trace == ndJsonDeserialize(TraceFile)

VARIABLE cursor

traceVars == <<s, cursor>>

TraceInit ==
    /\ Init
    /\ cursor = 1
    /\ Len(Trace) >= 1
    /\ s = Trace[1].state.s

TraceStep ==
    /\ cursor < Len(Trace)
    /\ LET row == Trace[cursor + 1]
       IN
          /\ "event" \in DOMAIN row
          /\ "args" \in DOMAIN row
          /\ row.args = <<>>
          /\ CASE row.event = "ReserveAttempt" -> ReserveAttempt
                [] row.event = "MarkDispatchStarted" -> MarkDispatchStarted
                [] row.event = "NativeAcceptOld" -> NativeAcceptOld
                [] row.event = "OldDeviceApplies" -> OldDeviceApplies
                [] row.event = "OldDeviceDoesNotApply" -> OldDeviceDoesNotApply
                [] row.event = "OldDevicePartiallyApplies" -> OldDevicePartiallyApplies
                [] row.event = "BeginEffectRecordCommit" -> BeginEffectRecordCommit
                [] row.event = "CommitEffectRecord" -> CommitEffectRecord
                [] row.event = "AbortEffectRecord" -> AbortEffectRecord
                [] row.event = "AcknowledgeStoreCommit" -> AcknowledgeStoreCommit
                [] row.event = "AcknowledgeStoreNoCommit" -> AcknowledgeStoreNoCommit
                [] row.event = "Crash" -> Crash
                [] row.event = "Restart" -> Restart
                [] row.event = "FenceControllerQueue" -> FenceControllerQueue
                [] row.event = "FenceDownstreamExecution" -> FenceDownstreamExecution
                [] row.event = "ResolvePersistence" -> ResolvePersistence
                [] row.event = "ObserveOldEffect" -> ObserveOldEffect
                [] row.event = "ObserveUnknownEffect" -> ObserveUnknownEffect
                [] row.event = "ObserveDisputedEffect" -> ObserveDisputedEffect
                [] row.event = "ResolveMission" -> ResolveMission
                [] row.event = "IssueRecoveryPermit" -> IssueRecoveryPermit
                [] row.event = "CreateFreshTicket" -> CreateFreshTicket
                [] row.event = "RevokeM1" -> RevokeM1
                [] row.event = "RevokeM2" -> RevokeM2
                [] row.event = "ChangeLiveBinding" -> ChangeLiveBinding
                [] row.event = "NativeAcceptRecovery" -> NativeAcceptRecovery
                [] row.event = "RecoveryDeviceApplies" -> RecoveryDeviceApplies
                [] row.event = "RecoveryDeviceDoesNotApply" -> RecoveryDeviceDoesNotApply
                [] row.event = "RecoveryDevicePartiallyApplies" -> RecoveryDevicePartiallyApplies
                [] row.event = "NaiveRetry" -> NaiveRetry
                [] OTHER -> FALSE
          /\ s' = row.state.s
    /\ cursor' = cursor + 1

TraceSpec ==
    TraceInit /\ [][TraceStep]_traceVars

\* If a recorded step cannot be matched by its named TLA action and exact
\* post-state, the cursor cannot advance and this invariant fails.
TraceNotStuck ==
    cursor = Len(Trace) \/ ENABLED TraceStep

====
