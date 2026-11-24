-------------------------- MODULE GazeboResetLifecycle --------------------------
EXTENDS Naturals, TLC

CONSTANT AllowUnsafeRetry

VARIABLES phase,
          resetRequests,
          resetApplied,
          initialEpoch,
          worldEpoch,
          initialState,
          observedEpoch,
          observedInitial,
          faultDispatches

vars == <<phase, resetRequests, resetApplied, initialEpoch, worldEpoch,
          initialState, observedEpoch, observedInitial, faultDispatches>>

Phases == {"NotAttempted", "DispatchStarted", "AwaitingVerification",
           "OutcomeUnresolved", "Verified"}

Init ==
    /\ phase = "NotAttempted"
    /\ resetRequests = 0
    /\ resetApplied = FALSE
    /\ initialEpoch = 0
    /\ worldEpoch = initialEpoch
    /\ initialState = FALSE
    /\ observedEpoch = -1
    /\ observedInitial = FALSE
    /\ faultDispatches = 0

BeginReset ==
    /\ phase = "NotAttempted"
    /\ resetRequests' = resetRequests + 1
    /\ phase' = "DispatchStarted"
    /\ UNCHANGED <<resetApplied, initialEpoch, worldEpoch, initialState,
                   observedEpoch, observedInitial, faultDispatches>>

ResetApplies ==
    /\ phase \in {"DispatchStarted", "OutcomeUnresolved"}
    /\ resetApplied = FALSE
    /\ resetApplied' = TRUE
    /\ worldEpoch' = worldEpoch + 1
    /\ initialState' = TRUE
    /\ UNCHANGED <<phase, resetRequests, initialEpoch, observedEpoch,
                   observedInitial, faultDispatches>>

AcceptResetResponse ==
    /\ phase = "DispatchStarted"
    /\ resetApplied
    /\ phase' = "AwaitingVerification"
    /\ UNCHANGED <<resetRequests, resetApplied, initialEpoch, worldEpoch,
                   initialState, observedEpoch, observedInitial, faultDispatches>>

LoseResetResponse ==
    /\ phase = "DispatchStarted"
    /\ phase' = "OutcomeUnresolved"
    /\ UNCHANGED <<resetRequests, resetApplied, initialEpoch, worldEpoch,
                   initialState, observedEpoch, observedInitial, faultDispatches>>

ObserveReset ==
    /\ phase = "AwaitingVerification"
    /\ observedEpoch' = worldEpoch
    /\ observedInitial' = initialState
    /\ IF resetApplied /\ worldEpoch > initialEpoch /\ initialState
          /\ observedEpoch' = worldEpoch /\ observedInitial' = TRUE
       THEN phase' = "Verified"
       ELSE phase' = "OutcomeUnresolved"
    /\ UNCHANGED <<resetRequests, resetApplied, initialEpoch, worldEpoch,
                   initialState, faultDispatches>>

ReconcileReset ==
    /\ phase = "OutcomeUnresolved"
    /\ observedEpoch' = worldEpoch
    /\ observedInitial' = initialState
    /\ IF resetApplied /\ worldEpoch > initialEpoch /\ initialState
          /\ observedEpoch' = worldEpoch /\ observedInitial' = TRUE
       THEN phase' = "Verified"
       ELSE phase' = "OutcomeUnresolved"
    /\ UNCHANGED <<resetRequests, resetApplied, initialEpoch, worldEpoch,
                   initialState, faultDispatches>>

UnsafeRetry ==
    /\ AllowUnsafeRetry
    /\ phase = "OutcomeUnresolved"
    /\ resetRequests = 1
    /\ resetRequests' = 2
    /\ phase' = "DispatchStarted"
    /\ UNCHANGED <<resetApplied, initialEpoch, worldEpoch, initialState,
                   observedEpoch, observedInitial, faultDispatches>>

FaultDispatch ==
    /\ phase = "Verified"
    /\ faultDispatches = 0
    /\ faultDispatches' = 1
    /\ UNCHANGED <<phase, resetRequests, resetApplied, initialEpoch,
                   worldEpoch, initialState, observedEpoch, observedInitial>>

Next ==
    \/ BeginReset
    \/ ResetApplies
    \/ AcceptResetResponse
    \/ LoseResetResponse
    \/ ObserveReset
    \/ ReconcileReset
    \/ UnsafeRetry
    \/ FaultDispatch

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ phase \in Phases
    /\ resetRequests \in 0..2
    /\ resetApplied \in BOOLEAN
    /\ initialEpoch \in Nat
    /\ worldEpoch \in Nat
    /\ initialState \in BOOLEAN
    /\ observedEpoch \in {-1} \cup Nat
    /\ observedInitial \in BOOLEAN
    /\ faultDispatches \in 0..1

AtMostOneResetRequest ==
    resetRequests <= 1

NoFaultDispatchBeforeVerified ==
    faultDispatches = 0 \/ phase = "Verified"

VerifiedRequiresObservedReset ==
    phase = "Verified"
    =>
    /\ resetApplied
    /\ worldEpoch > initialEpoch
    /\ initialState
    /\ observedEpoch = worldEpoch
    /\ observedInitial = TRUE

=============================================================================
