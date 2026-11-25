-------------------------- MODULE JournaledMutatorCrashCut --------------------------
EXTENDS Naturals, TLC

CONSTANTS AllowUnsafeRedispatch,
          AllowFalseNoEffect,
          OwnerLedgerDurable

VARIABLES coordinatorUp,
          bootEpoch,
          journalIntent,
          journalOutcome,
          ownerLedger,
          ownerHistoryLost,
          requestInFlight,
          transportResult,
          dispatchCalls,
          physicalEffects,
          lastCallBootEpoch,
          recoveryGate

vars ==
    <<coordinatorUp, bootEpoch, journalIntent, journalOutcome, ownerLedger,
      ownerHistoryLost, requestInFlight, transportResult, dispatchCalls,
      physicalEffects, lastCallBootEpoch, recoveryGate>>

JournalOutcomes ==
    {"none", "accepted", "rejected", "unknown",
     "no_dispatch_proven", "payload_conflict"}

OwnerStates == {"absent", "applied", "no_effect", "payload_conflict"}

TransportResults == {"none", "accepted", "rejected", "unknown"}

RecoveryGates == {"fresh", "blocked", "reconciled"}

Init ==
    /\ coordinatorUp = TRUE
    /\ bootEpoch = 0
    /\ journalIntent = FALSE
    /\ journalOutcome = "none"
    /\ ownerLedger = "absent"
    /\ ownerHistoryLost = FALSE
    /\ requestInFlight = FALSE
    /\ transportResult = "none"
    /\ dispatchCalls = 0
    /\ physicalEffects = 0
    /\ lastCallBootEpoch = -1
    /\ recoveryGate = "fresh"

PersistIntent ==
    /\ coordinatorUp
    /\ bootEpoch = 0
    /\ ~journalIntent
    /\ journalIntent' = TRUE
    /\ UNCHANGED <<coordinatorUp, bootEpoch, journalOutcome, ownerLedger,
                   ownerHistoryLost, requestInFlight, transportResult,
                   dispatchCalls, physicalEffects, lastCallBootEpoch,
                   recoveryGate>>

DispatchApply ==
    /\ coordinatorUp
    /\ journalIntent
    /\ recoveryGate = "fresh"
    /\ dispatchCalls = 0
    /\ ownerLedger = "absent"
    /\ dispatchCalls' = dispatchCalls + 1
    /\ physicalEffects' = physicalEffects + 1
    /\ ownerLedger' = "applied"
    /\ requestInFlight' = TRUE
    /\ transportResult' = "none"
    /\ lastCallBootEpoch' = bootEpoch
    /\ UNCHANGED <<coordinatorUp, bootEpoch, journalIntent, journalOutcome,
                   ownerHistoryLost, recoveryGate>>

DispatchNoEffect ==
    /\ coordinatorUp
    /\ journalIntent
    /\ recoveryGate = "fresh"
    /\ dispatchCalls = 0
    /\ ownerLedger = "absent"
    /\ dispatchCalls' = dispatchCalls + 1
    /\ ownerLedger' = "no_effect"
    /\ requestInFlight' = TRUE
    /\ transportResult' = "none"
    /\ lastCallBootEpoch' = bootEpoch
    /\ UNCHANGED <<coordinatorUp, bootEpoch, journalIntent, journalOutcome,
                   ownerHistoryLost, physicalEffects, recoveryGate>>

PersistAcceptedResponse ==
    /\ coordinatorUp
    /\ requestInFlight
    /\ ownerLedger = "applied"
    /\ journalOutcome' = "accepted"
    /\ transportResult' = "accepted"
    /\ requestInFlight' = FALSE
    /\ UNCHANGED <<coordinatorUp, bootEpoch, journalIntent, ownerLedger,
                   ownerHistoryLost, dispatchCalls, physicalEffects,
                   lastCallBootEpoch, recoveryGate>>

PersistRejectedResponse ==
    /\ coordinatorUp
    /\ requestInFlight
    /\ ownerLedger \in {"applied", "no_effect"}
    /\ journalOutcome' = "rejected"
    /\ transportResult' = "rejected"
    /\ requestInFlight' = FALSE
    /\ UNCHANGED <<coordinatorUp, bootEpoch, journalIntent, ownerLedger,
                   ownerHistoryLost, dispatchCalls, physicalEffects,
                   lastCallBootEpoch, recoveryGate>>

DropResponse ==
    /\ coordinatorUp
    /\ requestInFlight
    /\ transportResult' = "unknown"
    /\ requestInFlight' = FALSE
    /\ UNCHANGED <<coordinatorUp, bootEpoch, journalIntent, journalOutcome,
                   ownerLedger, ownerHistoryLost, dispatchCalls, physicalEffects,
                   lastCallBootEpoch, recoveryGate>>

CoordinatorCrash ==
    /\ coordinatorUp
    /\ bootEpoch = 0
    /\ coordinatorUp' = FALSE
    /\ requestInFlight' = FALSE
    /\ ownerLedger' =
         IF OwnerLedgerDurable THEN ownerLedger ELSE "absent"
    /\ ownerHistoryLost' =
         ownerHistoryLost \/
         (~OwnerLedgerDurable /\ ownerLedger # "absent")
    /\ recoveryGate' =
         IF journalIntent THEN "blocked" ELSE "fresh"
    /\ UNCHANGED <<bootEpoch, journalIntent, journalOutcome, transportResult,
                   dispatchCalls, physicalEffects, lastCallBootEpoch>>

CoordinatorRestart ==
    /\ ~coordinatorUp
    /\ bootEpoch = 0
    /\ coordinatorUp' = TRUE
    /\ bootEpoch' = 1
    /\ recoveryGate' =
         IF journalIntent THEN "blocked" ELSE "fresh"
    /\ UNCHANGED <<journalIntent, journalOutcome, ownerLedger, ownerHistoryLost,
                   requestInFlight, transportResult, dispatchCalls,
                   physicalEffects, lastCallBootEpoch>>

ReconcileOwnerLedger ==
    /\ coordinatorUp
    /\ bootEpoch = 1
    /\ journalIntent
    /\ recoveryGate = "blocked"
    /\ recoveryGate' = "reconciled"
    /\ journalOutcome' =
         IF journalOutcome \in {"accepted", "no_dispatch_proven", "payload_conflict"}
         THEN journalOutcome
         ELSE IF ownerHistoryLost
              THEN "unknown"
              ELSE IF ownerLedger = "applied"
                   THEN "accepted"
                   ELSE IF ownerLedger = "payload_conflict"
                        THEN "payload_conflict"
                        ELSE "no_dispatch_proven"
    /\ UNCHANGED <<coordinatorUp, bootEpoch, journalIntent, ownerLedger,
                   ownerHistoryLost, requestInFlight, transportResult,
                   dispatchCalls, physicalEffects, lastCallBootEpoch>>

UnsafeRedispatch ==
    /\ AllowUnsafeRedispatch
    /\ coordinatorUp
    /\ bootEpoch = 1
    /\ journalIntent
    /\ recoveryGate \in {"blocked", "reconciled"}
    /\ dispatchCalls < 2
    /\ dispatchCalls' = dispatchCalls + 1
    /\ lastCallBootEpoch' = bootEpoch
    /\ IF ownerLedger = "absent"
       THEN
         /\ ownerLedger' = "applied"
         /\ physicalEffects' = physicalEffects + 1
       ELSE
         /\ UNCHANGED <<ownerLedger, physicalEffects>>
    /\ UNCHANGED <<coordinatorUp, bootEpoch, journalIntent, journalOutcome,
                   ownerHistoryLost, requestInFlight, transportResult,
                   recoveryGate>>

UnsafeInferNoEffectFromRejectedResponse ==
    /\ AllowFalseNoEffect
    /\ coordinatorUp
    /\ journalIntent
    /\ journalOutcome = "rejected"
    /\ journalOutcome' = "no_dispatch_proven"
    /\ UNCHANGED <<coordinatorUp, bootEpoch, journalIntent, ownerLedger,
                   ownerHistoryLost, requestInFlight, transportResult,
                   dispatchCalls, physicalEffects, lastCallBootEpoch,
                   recoveryGate>>

Next ==
    \/ PersistIntent
    \/ DispatchApply
    \/ DispatchNoEffect
    \/ PersistAcceptedResponse
    \/ PersistRejectedResponse
    \/ DropResponse
    \/ CoordinatorCrash
    \/ CoordinatorRestart
    \/ ReconcileOwnerLedger
    \/ UnsafeRedispatch
    \/ UnsafeInferNoEffectFromRejectedResponse

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ coordinatorUp \in BOOLEAN
    /\ bootEpoch \in 0..1
    /\ journalIntent \in BOOLEAN
    /\ journalOutcome \in JournalOutcomes
    /\ ownerLedger \in OwnerStates
    /\ ownerHistoryLost \in BOOLEAN
    /\ requestInFlight \in BOOLEAN
    /\ transportResult \in TransportResults
    /\ dispatchCalls \in 0..2
    /\ physicalEffects \in 0..2
    /\ lastCallBootEpoch \in {-1, 0, 1}
    /\ recoveryGate \in RecoveryGates

NoMutationBeforeDurableIntent ==
    dispatchCalls = 0 \/ journalIntent

AtMostOneDispatchCallPerAttempt ==
    dispatchCalls <= 1

AtMostOnePhysicalEffectPerAttempt ==
    physicalEffects <= 1

NoFalseNoEffect ==
    journalOutcome = "no_dispatch_proven"
    =>
    /\ ownerLedger \in {"absent", "no_effect"}
    /\ ~ownerHistoryLost

NoMutationAfterRestartForOldIntent ==
    lastCallBootEpoch # 1

NoDispatchWhileRecoveryBlocked ==
    ~(lastCallBootEpoch = 1 /\ recoveryGate = "blocked")

=============================================================================
