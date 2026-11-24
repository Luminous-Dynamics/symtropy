-------------------------- MODULE GazeboResetLifecycle --------------------------
EXTENDS Naturals, TLC

CONSTANTS AllowUnsafeRetry, AllowPlanSubstitution

VARIABLES phase,
          resetRequests,
          resetApplied,
          initialEpoch,
          worldEpoch,
          initialState,
          observedEpoch,
          observedInitial,
          faultDispatches,
          activePlan,
          resetPlan,
          dispatchedPlan

vars == <<phase, resetRequests, resetApplied, initialEpoch, worldEpoch,
          initialState, observedEpoch, observedInitial, faultDispatches,
          activePlan, resetPlan, dispatchedPlan>>

Phases == {"NotAttempted", "DispatchStarted", "AwaitingVerification",
           "OutcomeUnresolved", "Verified"}

PlanIds == {"plan-a", "plan-b"}

Init ==
    /\ phase = "NotAttempted"
    /\ resetRequests = 0
    /\ resetApplied = FALSE
    /\ initialEpoch = 0
    /\ worldEpoch = initialEpoch
    /\ initialState = FALSE
    /\ observedEpoch = 0
    /\ observedInitial = FALSE
    /\ faultDispatches = 0
    /\ activePlan = "plan-a"
    /\ resetPlan = "none"
    /\ dispatchedPlan = "none"

BeginReset ==
    /\ phase = "NotAttempted"
    /\ resetRequests' = resetRequests + 1
    /\ phase' = "DispatchStarted"
    /\ resetPlan' = activePlan
    /\ UNCHANGED <<resetApplied, initialEpoch, worldEpoch, initialState,
                   observedEpoch, observedInitial, faultDispatches,
                   activePlan, dispatchedPlan>>

ResetApplies ==
    /\ phase \in {"DispatchStarted", "OutcomeUnresolved"}
    /\ resetApplied = FALSE
    /\ resetApplied' = TRUE
    /\ worldEpoch' = worldEpoch + 1
    /\ initialState' = TRUE
    /\ UNCHANGED <<phase, resetRequests, initialEpoch, observedEpoch,
                   observedInitial, faultDispatches, activePlan, resetPlan,
                   dispatchedPlan>>

AcceptResetResponse ==
    /\ phase = "DispatchStarted"
    /\ resetApplied
    /\ phase' = "AwaitingVerification"
    /\ UNCHANGED <<resetRequests, resetApplied, initialEpoch, worldEpoch,
                   initialState, observedEpoch, observedInitial, faultDispatches,
                   activePlan, resetPlan, dispatchedPlan>>

LoseResetResponse ==
    /\ phase = "DispatchStarted"
    /\ phase' = "OutcomeUnresolved"
    /\ UNCHANGED <<resetRequests, resetApplied, initialEpoch, worldEpoch,
                   initialState, observedEpoch, observedInitial, faultDispatches,
                   activePlan, resetPlan, dispatchedPlan>>

ObserveReset ==
    /\ phase = "AwaitingVerification"
    /\ observedEpoch' = worldEpoch
    /\ observedInitial' = initialState
    /\ IF resetApplied /\ worldEpoch > initialEpoch /\ initialState
       THEN phase' = "Verified"
       ELSE phase' = "OutcomeUnresolved"
    /\ UNCHANGED <<resetRequests, resetApplied, initialEpoch, worldEpoch,
                   initialState, faultDispatches, activePlan, resetPlan,
                   dispatchedPlan>>

ReconcileReset ==
    /\ phase = "OutcomeUnresolved"
    /\ observedEpoch' = worldEpoch
    /\ observedInitial' = initialState
    /\ IF resetApplied /\ worldEpoch > initialEpoch /\ initialState
       THEN phase' = "Verified"
       ELSE phase' = "OutcomeUnresolved"
    /\ UNCHANGED <<resetRequests, resetApplied, initialEpoch, worldEpoch,
                   initialState, faultDispatches, activePlan, resetPlan,
                   dispatchedPlan>>

UnsafeRetry ==
    /\ AllowUnsafeRetry
    /\ phase = "OutcomeUnresolved"
    /\ resetRequests = 1
    /\ resetRequests' = 2
    /\ phase' = "DispatchStarted"
    /\ UNCHANGED <<resetApplied, initialEpoch, worldEpoch, initialState,
                   observedEpoch, observedInitial, faultDispatches,
                   activePlan, resetPlan, dispatchedPlan>>

UnsafePlanSubstitution ==
    /\ AllowPlanSubstitution
    /\ phase = "Verified"
    /\ faultDispatches = 0
    /\ activePlan = "plan-a"
    /\ activePlan' = "plan-b"
    /\ UNCHANGED <<phase, resetRequests, resetApplied, initialEpoch,
                   worldEpoch, initialState, observedEpoch, observedInitial,
                   faultDispatches, resetPlan, dispatchedPlan>>

FaultDispatch ==
    /\ phase = "Verified"
    /\ faultDispatches = 0
    /\ faultDispatches' = 1
    /\ dispatchedPlan' = activePlan
    /\ UNCHANGED <<phase, resetRequests, resetApplied, initialEpoch,
                   worldEpoch, initialState, observedEpoch, observedInitial,
                   activePlan, resetPlan>>

Next ==
    \/ BeginReset
    \/ ResetApplies
    \/ AcceptResetResponse
    \/ LoseResetResponse
    \/ ObserveReset
    \/ ReconcileReset
    \/ UnsafeRetry
    \/ UnsafePlanSubstitution
    \/ FaultDispatch

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ phase \in Phases
    /\ resetRequests \in 0..2
    /\ resetApplied \in BOOLEAN
    /\ initialEpoch \in Nat
    /\ worldEpoch \in Nat
    /\ initialState \in BOOLEAN
    /\ observedEpoch \in Nat
    /\ observedInitial \in BOOLEAN
    /\ faultDispatches \in 0..1
    /\ activePlan \in PlanIds
    /\ resetPlan \in {"none"} \cup PlanIds
    /\ dispatchedPlan \in {"none"} \cup PlanIds

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

DispatchUsesResetPlan ==
    faultDispatches = 0 \/ dispatchedPlan = resetPlan

=============================================================================
